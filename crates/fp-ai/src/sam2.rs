//! SAM 2.1 (Meta, „Segment Anything in Images and Videos“, Apache-2.0) für Einzelbilder als eigene
//! Umsetzung für die CPU: Hiera-Bildencoder mit FPN-Hals, Prompt-Encoder (Punkte, Rahmen,
//! Masken-Eingabe) und Maskendecoder mit hochauflösenden Merkmalen. Gewichte: die offiziellen
//! `model.safetensors` von `facebook/sam2.1-hiera-tiny` bzw. `-small`.
//!
//! Ablauf: [`Sam2::encode`] einmal je Bild (teuer), danach [`Sam2::decode`] je Klick (schnell).

use crate::cancel::Progress;
use crate::error::{AiError, Result};
use crate::nn::{self, Conv2d};
use crate::weights::{Linear, Norm, Weights};
use rayon::prelude::*;

/// Kantenlänge der Modell-Eingabe.
pub const SIZE: usize = 1024;
/// Kantenlänge der Bild-Einbettung (SIZE / 16).
pub const EMB: usize = 64;
/// Kantenlänge der Masken-Logits (SIZE / 4).
pub const LOW: usize = 256;

/// Aufbau des Hiera-Encoders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HieraConfig {
    pub blocks_per_stage: Vec<usize>,
    pub embed: Vec<usize>,
    pub heads: Vec<usize>,
    pub window: Vec<usize>,
    pub global: Vec<usize>,
}

impl HieraConfig {
    /// Hiera-T (sam2.1-hiera-tiny).
    pub fn tiny() -> HieraConfig {
        HieraConfig { blocks_per_stage: vec![1, 2, 7, 2], embed: vec![96, 192, 384, 768], heads: vec![1, 2, 4, 8], window: vec![8, 4, 14, 7], global: vec![5, 7, 9] }
    }
    /// Hiera-S (sam2.1-hiera-small).
    pub fn small() -> HieraConfig {
        HieraConfig { blocks_per_stage: vec![1, 2, 11, 2], embed: vec![96, 192, 384, 768], heads: vec![1, 2, 4, 8], window: vec![8, 4, 14, 7], global: vec![7, 10, 13] }
    }
}

struct HieraBlock {
    norm1: Norm,
    qkv: Linear,
    proj: Linear,
    norm2: Norm,
    mlp_in: Linear,
    mlp_out: Linear,
    dim_proj: Option<Linear>,
    dim: usize,
    dim_out: usize,
    heads: usize,
    window: usize,
    q_stride: bool,
}

struct Attn {
    q: Linear,
    k: Linear,
    v: Linear,
    o: Linear,
    heads: usize,
}

impl Attn {
    fn load(wt: &mut Weights, p: &str, heads: usize) -> Result<Attn> {
        Ok(Attn { q: wt.linear(&format!("{p}.q_proj"))?, k: wt.linear(&format!("{p}.k_proj"))?, v: wt.linear(&format!("{p}.v_proj"))?, o: wt.linear(&format!("{p}.o_proj"))?, heads })
    }
    /// `q[nq, 256]`, `k`/`v[nk, 256]` → `[nq, 256]`.
    fn forward(&self, q: &[f32], nq: usize, k: &[f32], v: &[f32], nk: usize) -> Vec<f32> {
        let qq = self.q.forward(q, nq);
        let kk = self.k.forward(k, nk);
        let vv = self.v.forward(v, nk);
        let internal = self.q.cout;
        let d = internal / self.heads;
        let a = nn::attention(&qq, &kk, &vv, 1, nq, nk, self.heads, d, (d as f32).powf(-0.5), None);
        nn::give(qq);
        nn::give(kk);
        nn::give(vv);
        let o = self.o.forward(&a, nq);
        nn::give(a);
        o
    }
}

struct TwoWayLayer {
    self_attn: Attn,
    n1: Norm,
    cross_t2i: Attn,
    n2: Norm,
    mlp_in: Linear,
    mlp_out: Linear,
    n3: Norm,
    n4: Norm,
    cross_i2t: Attn,
    skip_first_pe: bool,
}

struct Mlp {
    layers: Vec<Linear>,
    sigmoid: bool,
}

impl Mlp {
    fn load(wt: &mut Weights, p: &str, n: usize, sigmoid: bool) -> Result<Mlp> {
        let mut layers = vec![wt.linear(&format!("{p}.proj_in"))?];
        for i in 0..n.saturating_sub(2) {
            layers.push(wt.linear(&format!("{p}.layers.{i}"))?);
        }
        layers.push(wt.linear(&format!("{p}.proj_out"))?);
        Ok(Mlp { layers, sigmoid })
    }
    fn forward(&self, x: &[f32], n: usize) -> Vec<f32> {
        let mut h = x.to_vec();
        let last = self.layers.len() - 1;
        for (i, l) in self.layers.iter().enumerate() {
            h = l.forward(&h, n);
            if i < last {
                for v in &mut h {
                    *v = v.max(0.0);
                }
            }
        }
        if self.sigmoid {
            for v in &mut h {
                *v = nn::sigmoid(*v);
            }
        }
        h
    }
}

/// Ergebnis des Bild-Encoders (zwischengespeichert je Bild).
#[derive(Clone, Debug)]
pub struct ImageEmbedding {
    /// `[256, 64, 64]` inkl. „keine Erinnerung“-Einbettung.
    pub emb: Vec<f32>,
    /// `[32, 256, 256]` (conv_s0 angewandt).
    pub s0: Vec<f32>,
    /// `[64, 128, 128]` (conv_s1 angewandt).
    pub s1: Vec<f32>,
}

/// Eingaben für den Decoder (in Modellkoordinaten 0…1024).
#[derive(Clone, Debug, Default)]
pub struct Prompt {
    /// Punkte (x, y) mit Label 1 (Objekt) bzw. 0 (Hintergrund).
    pub points: Vec<(f32, f32, i32)>,
    /// Rahmen (x0, y0, x1, y1).
    pub boxx: Option<[f32; 4]>,
    /// Vorherige Masken-Logits `[256, 256]` als zusätzliche Eingabe (Verfeinern).
    pub mask_input: Option<Vec<f32>>,
    /// Drei Masken ausgeben und die beste wählen (bei einem einzelnen Punkt).
    pub multimask: bool,
}

/// Ergebnis des Decoders.
#[derive(Clone, Debug)]
pub struct Decoded {
    /// Masken-Logits `[256, 256]` (> 0 = Objekt).
    pub logits: Vec<f32>,
    /// Vorhergesagte Güte (IoU).
    pub iou: f32,
    /// Objekt-Wert (Logit; < 0 = vermutlich kein Objekt).
    pub object_score: f32,
}

/// Geladenes SAM 2.1.
pub struct Sam2 {
    cfg: HieraConfig,
    patch: Conv2d,
    /// Positions-Einbettung `[256·256, 96]` (bikubisch skaliert + Fenstermuster).
    pos: Vec<f32>,
    blocks: Vec<HieraBlock>,
    neck: Vec<Conv2d>,
    conv_s0: Conv2d,
    conv_s1: Conv2d,
    no_mem: Vec<f32>,
    // Prompt-Encoder
    pe_gauss: Vec<f32>,
    point_embed: Vec<f32>,
    not_a_point: Vec<f32>,
    no_mask: Vec<f32>,
    mask_conv1: Conv2d,
    mask_n1: Norm,
    mask_conv2: Conv2d,
    mask_n2: Norm,
    mask_conv3: Conv2d,
    // Decoder
    tokens: Vec<f32>,
    layers: Vec<TwoWayLayer>,
    final_attn: Attn,
    final_norm: Norm,
    up1_w: Vec<f32>,
    up1_b: Vec<f32>,
    up_norm: Norm,
    up2_w: Vec<f32>,
    up2_b: Vec<f32>,
    hyper: Vec<Mlp>,
    iou_head: Mlp,
    obj_head: Mlp,
    image_pe: Vec<f32>,
}

impl Sam2 {
    /// Aus Gewichten aufbauen (Größe am Gewichtsbestand erkannt).
    pub fn load(mut wt: Weights) -> Result<Sam2> {
        let mut n_blocks = 0;
        while wt.has(&format!("vision_encoder.backbone.blocks.{n_blocks}.layer_norm1.weight")) {
            n_blocks += 1;
        }
        let cfg = match n_blocks {
            12 => HieraConfig::tiny(),
            16 => HieraConfig::small(),
            n => return Err(AiError::Weights(format!("unbekannte SAM-2-Größe ({n} Blöcke)"))),
        };
        let bb = "vision_encoder.backbone";
        let patch = wt.conv(&format!("{bb}.patch_embed.projection"), 4, 3, 1)?;
        let c0 = cfg.embed[0];
        let pe = wt.vec(&format!("{bb}.pos_embed"))?; // [1, 96, 7, 7]
        let pw = wt.vec(&format!("{bb}.pos_embed_window"))?; // [1, 96, 8, 8]
        let g = SIZE / 4;
        let big = nn::resize_bicubic(&pe, c0, 7, 7, g, g);
        let wsz = cfg.window[0];
        let mut pos = vec![0.0f32; g * g * c0];
        for y in 0..g {
            for x in 0..g {
                for c in 0..c0 {
                    pos[(y * g + x) * c0 + c] = big[c * g * g + y * g + x] + pw[c * wsz * wsz + (y % wsz) * wsz + (x % wsz)];
                }
            }
        }
        let mut blocks = Vec::new();
        let mut total = 0;
        for (s, &nb) in cfg.blocks_per_stage.iter().enumerate() {
            for b in 0..nb {
                let dim = if s > 0 && b == 0 { cfg.embed[s - 1] } else { cfg.embed[s] };
                let dim_out = cfg.embed[s];
                let mut window = if s > 0 && b == 0 { cfg.window[s - 1] } else { cfg.window[s] };
                if cfg.global.contains(&total) {
                    window = 0;
                }
                let q_stride = s > 0 && s <= 3 && b == 0;
                let p = format!("{bb}.blocks.{total}");
                blocks.push(HieraBlock {
                    norm1: wt.norm(&format!("{p}.layer_norm1"), 1e-6)?,
                    qkv: wt.linear(&format!("{p}.attn.qkv"))?,
                    proj: wt.linear(&format!("{p}.attn.proj"))?,
                    norm2: wt.norm(&format!("{p}.layer_norm2"), 1e-6)?,
                    mlp_in: wt.linear(&format!("{p}.mlp.proj_in"))?,
                    mlp_out: wt.linear(&format!("{p}.mlp.proj_out"))?,
                    dim_proj: if dim != dim_out { Some(wt.linear(&format!("{p}.proj"))?) } else { None },
                    dim,
                    dim_out,
                    heads: cfg.heads[s],
                    window,
                    q_stride,
                });
                total += 1;
            }
        }
        let mut neck = Vec::new();
        for i in 0..4 {
            neck.push(wt.conv(&format!("vision_encoder.neck.convs.{i}"), 1, 0, 1)?);
        }
        let md = "mask_decoder";
        let conv_s0 = wt.conv(&format!("{md}.conv_s0"), 1, 0, 1)?;
        let conv_s1 = wt.conv(&format!("{md}.conv_s1"), 1, 0, 1)?;
        let no_mem = wt.vec("no_memory_embedding")?;
        let pe_gauss = wt.vec("prompt_encoder.shared_embedding.positional_embedding")?;
        let shared = wt.opt("shared_image_embedding.positional_embedding").unwrap_or_else(|| pe_gauss.clone());
        let pr = "prompt_encoder";
        let point_embed = wt.vec(&format!("{pr}.point_embed.weight"))?;
        let not_a_point = wt.vec(&format!("{pr}.not_a_point_embed.weight"))?;
        let no_mask = wt.vec(&format!("{pr}.no_mask_embed.weight"))?;
        let mask_conv1 = wt.conv(&format!("{pr}.mask_embed.conv1"), 2, 0, 1)?;
        let mask_n1 = wt.norm(&format!("{pr}.mask_embed.layer_norm1"), 1e-6)?;
        let mask_conv2 = wt.conv(&format!("{pr}.mask_embed.conv2"), 2, 0, 1)?;
        let mask_n2 = wt.norm(&format!("{pr}.mask_embed.layer_norm2"), 1e-6)?;
        let mask_conv3 = wt.conv(&format!("{pr}.mask_embed.conv3"), 1, 0, 1)?;
        let mut tokens = wt.vec(&format!("{md}.obj_score_token.weight"))?;
        tokens.extend(wt.vec(&format!("{md}.iou_token.weight"))?);
        tokens.extend(wt.vec(&format!("{md}.mask_tokens.weight"))?);
        let mut layers = Vec::new();
        for i in 0..2 {
            let p = format!("{md}.transformer.layers.{i}");
            layers.push(TwoWayLayer {
                self_attn: Attn::load(&mut wt, &format!("{p}.self_attn"), 8)?,
                n1: wt.norm(&format!("{p}.layer_norm1"), 1e-5)?,
                cross_t2i: Attn::load(&mut wt, &format!("{p}.cross_attn_token_to_image"), 8)?,
                n2: wt.norm(&format!("{p}.layer_norm2"), 1e-5)?,
                mlp_in: wt.linear(&format!("{p}.mlp.proj_in"))?,
                mlp_out: wt.linear(&format!("{p}.mlp.proj_out"))?,
                n3: wt.norm(&format!("{p}.layer_norm3"), 1e-5)?,
                n4: wt.norm(&format!("{p}.layer_norm4"), 1e-5)?,
                cross_i2t: Attn::load(&mut wt, &format!("{p}.cross_attn_image_to_token"), 8)?,
                skip_first_pe: i == 0,
            });
        }
        let final_attn = Attn::load(&mut wt, &format!("{md}.transformer.final_attn_token_to_image"), 8)?;
        let final_norm = wt.norm(&format!("{md}.transformer.layer_norm_final_attn"), 1e-5)?;
        let up1_w = wt.vec(&format!("{md}.upscale_conv1.weight"))?;
        let up1_b = wt.vec(&format!("{md}.upscale_conv1.bias"))?;
        let up_norm = wt.norm(&format!("{md}.upscale_layer_norm"), 1e-6)?;
        let up2_w = wt.vec(&format!("{md}.upscale_conv2.weight"))?;
        let up2_b = wt.vec(&format!("{md}.upscale_conv2.bias"))?;
        let mut hyper = Vec::new();
        for i in 0..4 {
            hyper.push(Mlp::load(&mut wt, &format!("{md}.output_hypernetworks_mlps.{i}"), 3, false)?);
        }
        let iou_head = Mlp::load(&mut wt, &format!("{md}.iou_prediction_head"), 3, true)?;
        let obj_head = Mlp::load(&mut wt, &format!("{md}.pred_obj_score_head"), 3, false)?;
        // Bildweite Positions-Einbettung [64·64, 256] (Token-Layout)
        let mut coords = Vec::with_capacity(EMB * EMB * 2);
        for y in 0..EMB {
            for x in 0..EMB {
                coords.push((x as f32 + 0.5) / EMB as f32);
                coords.push((y as f32 + 0.5) / EMB as f32);
            }
        }
        let image_pe = fourier(&coords, &shared);
        Ok(Sam2 {
            cfg,
            patch,
            pos,
            blocks,
            neck,
            conv_s0,
            conv_s1,
            no_mem,
            pe_gauss,
            point_embed,
            not_a_point,
            no_mask,
            mask_conv1,
            mask_n1,
            mask_conv2,
            mask_n2,
            mask_conv3,
            tokens,
            layers,
            final_attn,
            final_norm,
            up1_w,
            up1_b,
            up_norm,
            up2_w,
            up2_b,
            hyper,
            iou_head,
            obj_head,
            image_pe,
        })
    }

    /// Aufbau des Encoders.
    pub fn config(&self) -> &HieraConfig {
        &self.cfg
    }

    /// Bild-Encoder: `x[3, 1024, 1024]` (normalisiert).
    pub fn encode(&self, x: &[f32], prog: &Progress) -> Result<ImageEmbedding> {
        if x.len() != 3 * SIZE * SIZE {
            return Err(AiError::Input("SAM 2 braucht ein Bild mit 1024 × 1024".into()));
        }
        let (pe, mut h, mut w) = self.patch.forward(x, SIZE, SIZE);
        let c0 = self.cfg.embed[0];
        let mut t = nn::transpose(&pe, c0, h * w);
        drop(pe);
        nn::add_inplace(&mut t, &self.pos);
        let ends: Vec<usize> = self.cfg.blocks_per_stage.iter().scan(0usize, |a, n| {
            *a += n;
            Some(*a - 1)
        }).collect();
        let mut stage_out: Vec<(Vec<f32>, usize, usize, usize)> = Vec::new();
        let nb = self.blocks.len();
        for (i, b) in self.blocks.iter().enumerate() {
            let (o, h2, w2) = hiera_block(b, &t, h, w);
            t = o;
            h = h2;
            w = w2;
            if ends.contains(&i) {
                stage_out.push((t.clone(), h, w, b.dim_out));
            }
            prog.step(0.0, 0.9, (i + 1) as f32 / nb as f32)?;
        }
        // FPN-Hals: Ebene i (0 = 256², 3 = 32²), convs[3 - i]
        let mut lat: Vec<Vec<f32>> = Vec::new();
        for (i, (tok, hh, ww, c)) in stage_out.iter().enumerate() {
            let chw = nn::transpose(tok, hh * ww, *c);
            lat.push(self.neck[3 - i].forward(&chw, *hh, *ww).0);
        }
        // Ebene 3 (32²) + Ebene 2 (64²) von oben nach unten; Ebenen 1 und 0 ohne Aufwärtspfad
        let (h3, w3) = (stage_out[3].1, stage_out[3].2);
        let up = nn::upsample_nearest_2x(&lat[3], 256, h3, w3);
        let mut l2 = lat[2].clone();
        nn::add_inplace(&mut l2, &up);
        // + „keine Erinnerung“
        let hw2 = stage_out[2].1 * stage_out[2].2;
        for (k, v) in self.no_mem.iter().enumerate() {
            for x in &mut l2[k * hw2..(k + 1) * hw2] {
                *x += *v;
            }
        }
        let s1 = self.conv_s1.forward(&lat[1], stage_out[1].1, stage_out[1].2).0;
        let s0 = self.conv_s0.forward(&lat[0], stage_out[0].1, stage_out[0].2).0;
        prog.step(0.9, 1.0, 1.0)?;
        Ok(ImageEmbedding { emb: l2, s0, s1 })
    }

    fn embed_mask(&self, m: &[f32]) -> Vec<f32> {
        let (a, h, w) = self.mask_conv1.forward(m, LOW, LOW);
        let mut a = a;
        self.mask_n1.channels(&mut a);
        nn::gelu_inplace(&mut a);
        let (b, h2, w2) = self.mask_conv2.forward(&a, h, w);
        let mut b = b;
        self.mask_n2.channels(&mut b);
        nn::gelu_inplace(&mut b);
        self.mask_conv3.forward(&b, h2, w2).0
    }

    /// Maskendecoder für eine Eingabe.
    pub fn decode(&self, e: &ImageEmbedding, p: &Prompt) -> Result<Decoded> {
        let d = 256usize;
        let tp = std::time::Instant::now();
        // --- Prompt-Token
        let mut sparse: Vec<f32> = Vec::new();
        let mut n_sparse = 0;
        let pad = p.boxx.is_none();
        let mut pts: Vec<(f32, f32, i32)> = p.points.clone();
        if pad {
            pts.push((0.0, 0.0, -1));
        }
        for (x, y, l) in &pts {
            let c = [(x + 0.5) / SIZE as f32, (y + 0.5) / SIZE as f32];
            let mut v = fourier(&c, &self.pe_gauss);
            if *l == -1 {
                v.copy_from_slice(&self.not_a_point);
            } else {
                let li = (*l).clamp(0, 3) as usize;
                for (a, b) in v.iter_mut().zip(&self.point_embed[li * d..(li + 1) * d]) {
                    *a += *b;
                }
            }
            sparse.extend(v);
            n_sparse += 1;
        }
        if let Some(b) = p.boxx {
            for (i, (x, y)) in [(b[0], b[1]), (b[2], b[3])].iter().enumerate() {
                let c = [(x + 0.5) / SIZE as f32, (y + 0.5) / SIZE as f32];
                let mut v = fourier(&c, &self.pe_gauss);
                for (a, bb) in v.iter_mut().zip(&self.point_embed[(2 + i) * d..(3 + i) * d]) {
                    *a += *bb;
                }
                sparse.extend(v);
                n_sparse += 1;
            }
            sparse.extend_from_slice(&self.not_a_point);
            n_sparse += 1;
        }
        if p.points.is_empty() && p.boxx.is_none() {
            return Err(AiError::Input("kein Punkt und kein Rahmen".into()));
        }
        let nt = 6 + n_sparse;
        let mut point_emb = self.tokens.clone();
        point_emb.extend(sparse);
        // --- Bild + dichte Eingabe
        let hw = EMB * EMB;
        let mut src = nn::take_copy(&e.emb);
        match &p.mask_input {
            Some(m) if m.len() == LOW * LOW => {
                let dense = self.embed_mask(m);
                nn::add_inplace(&mut src, &dense);
            }
            _ => {
                for (k, v) in self.no_mask.iter().enumerate() {
                    for x in &mut src[k * hw..(k + 1) * hw] {
                        *x += *v;
                    }
                }
            }
        }
        let tp = nn::prof("dec prompt+dense", tp);
        let mut keys = nn::transpose(&src, d, hw); // [4096, 256]
        nn::give(src);
        let kpe = &self.image_pe;
        let mut queries = point_emb.clone();
        let qpe = &point_emb;
        for l in &self.layers {
            if l.skip_first_pe {
                queries = l.self_attn.forward(&queries, nt, &queries, &queries, nt);
            } else {
                let q = nn::add(&queries, qpe);
                let a = l.self_attn.forward(&q, nt, &q, &queries, nt);
                nn::add_inplace(&mut queries, &a);
            }
            l.n1.rows(&mut queries);
            let q = nn::add(&queries, qpe);
            let k = nn::add(&keys, kpe);
            let a = l.cross_t2i.forward(&q, nt, &k, &keys, hw);
            nn::add_inplace(&mut queries, &a);
            l.n2.rows(&mut queries);
            let mut m = l.mlp_in.forward(&queries, nt);
            nn::relu_inplace(&mut m);
            let m = l.mlp_out.forward(&m, nt);
            nn::add_inplace(&mut queries, &m);
            l.n3.rows(&mut queries);
            let q = nn::add(&queries, qpe);
            let a = l.cross_i2t.forward(&k, hw, &q, &queries, nt);
            nn::give(k);
            nn::add_inplace(&mut keys, &a);
            nn::give(a);
            l.n4.rows(&mut keys);
            nn::prof("dec layer", std::time::Instant::now());
        }
        let tp = nn::prof("dec transformer", tp);
        let q = nn::add(&queries, qpe);
        let k = nn::add(&keys, kpe);
        let a = self.final_attn.forward(&q, nt, &k, &keys, hw);
        nn::give(k);
        nn::add_inplace(&mut queries, &a);
        self.final_norm.rows(&mut queries);
        let tp = nn::prof("dec final attn", tp);
        // --- Masken
        let img = nn::transpose(&keys, hw, d); // [256, 64, 64]
        nn::give(keys);
        let mut u1 = nn::conv_transpose_2x2(&img, d, EMB, EMB, &self.up1_w, &self.up1_b, d / 4);
        nn::give(img);
        nn::add_inplace(&mut u1, &e.s1);
        self.up_norm.channels(&mut u1);
        nn::gelu_inplace(&mut u1);
        let mut u2 = nn::conv_transpose_2x2(&u1, d / 4, 2 * EMB, 2 * EMB, &self.up2_w, &self.up2_b, d / 8);
        nn::give(u1);
        nn::add_inplace(&mut u2, &e.s0);
        nn::gelu_inplace(&mut u2);
        let tp = nn::prof("dec upscale", tp);
        let mut hyper = Vec::with_capacity(4 * 32);
        for (i, mlp) in self.hyper.iter().enumerate() {
            hyper.extend(mlp.forward(&queries[(2 + i) * d..(3 + i) * d], 1));
        }
        let npx = LOW * LOW;
        let mut masks = nn::take(4 * npx);
        nn::gemm(&hyper, 4, 32, &u2, npx, false, &mut masks, false, true);
        nn::give(u2);
        let tp = nn::prof("dec masks", tp);
        let iou = self.iou_head.forward(&queries[d..2 * d], 1);
        let obj = self.obj_head.forward(&queries[..d], 1);
        let object_score = obj.first().copied().unwrap_or(0.0);
        if std::env::var_os("FP_AI_PROFILE").is_some() {
            let areas: Vec<usize> = (0..4).map(|i| masks[i * npx..(i + 1) * npx].iter().filter(|v| **v > 0.0).count()).collect();
            eprintln!("[fp-ai] SAM-Masken: iou {iou:?} Flächen {areas:?} obj {object_score}");
        }
        let pick = if p.multimask {
            (1..4).max_by(|a, b| iou[*a].total_cmp(&iou[*b])).unwrap_or(1)
        } else {
            // dynamische Wahl über die Stabilität der Einzelmaske
            let m0 = &masks[..npx];
            let inter = m0.iter().filter(|v| **v > 0.05).count();
            let union = m0.iter().filter(|v| **v > -0.05).count();
            let stab = if union > 0 { inter as f32 / union as f32 } else { 1.0 };
            if stab >= 0.98 { 0 } else { (1..4).max_by(|a, b| iou[*a].total_cmp(&iou[*b])).unwrap_or(1) }
        };
        nn::prof("dec rest", tp);
        let logits = masks[pick * npx..(pick + 1) * npx].to_vec();
        nn::give(masks);
        Ok(Decoded { logits, iou: iou[pick], object_score })
    }
}

/// Zufalls-Fourier-Merkmale: Koordinaten (0…1, Paare x/y) → `[n, 256]` (sin | cos).
fn fourier(coords: &[f32], g: &[f32]) -> Vec<f32> {
    let half = g.len() / 2;
    let n = coords.len() / 2;
    let mut o = vec![0.0f32; n * 2 * half];
    o.par_chunks_mut(2 * half).enumerate().for_each(|(i, row)| {
        let x = 2.0 * coords[2 * i] - 1.0;
        let y = 2.0 * coords[2 * i + 1] - 1.0;
        for j in 0..half {
            let v = 2.0 * std::f32::consts::PI * (x * g[j] + y * g[half + j]);
            row[j] = v.sin();
            row[half + j] = v.cos();
        }
    });
    o
}

/// Fenster bilden: Token `[H, W, C]` → `[nW, ws, ws, C]` (Rand mit Nullen).
fn partition(x: &[f32], h: usize, w: usize, c: usize, ws: usize) -> (Vec<f32>, usize, usize) {
    let hp = h.div_ceil(ws) * ws;
    let wp = w.div_ceil(ws) * ws;
    let (ny, nx) = (hp / ws, wp / ws);
    let mut o = vec![0.0f32; ny * nx * ws * ws * c];
    o.par_chunks_mut(ws * c).enumerate().for_each(|(r, row)| {
        // r = ((wy * nx + wx) * ws + ty)
        let ty = r % ws;
        let win = r / ws;
        let (wy, wx) = (win / nx, win % nx);
        let y = wy * ws + ty;
        if y >= h {
            return;
        }
        for tx in 0..ws {
            let xx = wx * ws + tx;
            if xx < w {
                row[tx * c..(tx + 1) * c].copy_from_slice(&x[(y * w + xx) * c..(y * w + xx + 1) * c]);
            }
        }
    });
    (o, hp, wp)
}

/// Umkehrung von [`partition`] (Rand verworfen).
fn unpartition(x: &[f32], h: usize, w: usize, c: usize, ws: usize, hp: usize, wp: usize) -> Vec<f32> {
    let nx = wp / ws;
    let _ = hp;
    let mut o = vec![0.0f32; h * w * c];
    o.par_chunks_mut(w * c).enumerate().for_each(|(y, row)| {
        let (wy, ty) = (y / ws, y % ws);
        for xx in 0..w {
            let (wx, tx) = (xx / ws, xx % ws);
            let src = (((wy * nx + wx) * ws + ty) * ws + tx) * c;
            row[xx * c..(xx + 1) * c].copy_from_slice(&x[src..src + c]);
        }
    });
    o
}

fn hiera_block(b: &HieraBlock, x: &[f32], h: usize, w: usize) -> (Vec<f32>, usize, usize) {
    let mut xn = x.to_vec();
    b.norm1.rows(&mut xn);
    let (mut residual, hr, wr) = if let Some(p) = &b.dim_proj {
        let pr = p.forward(&xn, h * w);
        if b.q_stride { nn::max_pool_2x2_nhwc(&pr, 1, h, w, b.dim_out) } else { (pr, h, w) }
    } else {
        (x.to_vec(), h, w)
    };
    // Fenster (0 = global)
    let (win, nb, ws, hp, wp) = if b.window > 0 {
        let (p, hp, wp) = partition(&xn, h, w, b.dim, b.window);
        let nb = (hp / b.window) * (wp / b.window);
        (p, nb, b.window, hp, wp)
    } else {
        (xn, 1, 0, h, w)
    };
    let (gh, gw) = if ws > 0 { (ws, ws) } else { (h, w) };
    let ntok = gh * gw;
    let qkv = b.qkv.forward(&win, nb * ntok);
    drop(win);
    let c = b.dim_out;
    let d = c / b.heads;
    let pk = |off: usize| nn::Packed { buf: &qkv, stride: 3 * c, off };
    let (att, nq) = if b.q_stride {
        // Anfragen herauslösen und je Fenster 2×2 max-poolen
        let mut q = vec![0.0f32; nb * ntok * c];
        q.par_chunks_mut(c).enumerate().for_each(|(i, qq)| qq.copy_from_slice(&qkv[i * 3 * c..i * 3 * c + c]));
        let (q, qh, qw) = nn::max_pool_2x2_nhwc(&q, nb, gh, gw, c);
        let nq = qh * qw;
        (nn::attention_packed(nn::Packed::dense(&q, c), pk(c), pk(2 * c), nb, nq, ntok, b.heads, d, (d as f32).powf(-0.5), None), nq)
    } else {
        (nn::attention_packed(pk(0), pk(c), pk(2 * c), nb, ntok, ntok, b.heads, d, (d as f32).powf(-0.5), None), ntok)
    };
    drop(qkv);
    let att = b.proj.forward(&att, nb * nq);
    // zurück ins Raster
    let attn_out = if ws > 0 {
        let ws2 = if b.q_stride { ws / 2 } else { ws };
        let (hp2, wp2) = (hr.div_ceil(ws2) * ws2, wr.div_ceil(ws2) * ws2);
        let _ = (hp, wp);
        unpartition(&att, hr, wr, c, ws2, hp2, wp2)
    } else {
        att
    };
    nn::add_inplace(&mut residual, &attn_out);
    drop(attn_out);
    let mut yn = residual.clone();
    b.norm2.rows(&mut yn);
    let mut m = b.mlp_in.forward(&yn, hr * wr);
    drop(yn);
    nn::gelu_inplace(&mut m);
    let m = b.mlp_out.forward(&m, hr * wr);
    nn::add_inplace(&mut residual, &m);
    (residual, hr, wr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_roundtrip() {
        let (h, w, c, ws) = (5, 7, 3, 4);
        let x: Vec<f32> = (0..h * w * c).map(|i| i as f32).collect();
        let (p, hp, wp) = partition(&x, h, w, c, ws);
        assert_eq!((hp, wp), (8, 8));
        assert_eq!(unpartition(&p, h, w, c, ws, hp, wp), x);
    }
}
