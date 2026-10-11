//! BiRefNet (Zheng et al., „Bilateral Reference for High-Resolution Dichotomous Image
//! Segmentation“, MIT-Lizenz) als eigene Umsetzung für die CPU: Swin-Transformer-Backbone (v1, T
//! oder L) mit Mehrskalen-Eingabe („cat“), Kontext aus allen Stufen, Decoder mit deformierbarem ASPP
//! und Gradienten-Aufmerksamkeit, Bildfelder als Decoder-Eingang. Gewichte: die offiziellen
//! `model.safetensors` von `ZhengPeng7/BiRefNet*`.
//!
//! Eingabe `[3, S, S]` (ImageNet-normalisiert), Ausgabe Logits `[S, S]` (Sigmoid = Alpha).

use crate::cancel::Progress;
use crate::error::{AiError, Result};
use crate::nn::{self, Conv2d};
use crate::weights::{Linear, Norm, Weights};

/// Größe eines Swin-v1-Backbones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwinConfig {
    pub embed: usize,
    pub depths: [usize; 4],
    pub heads: [usize; 4],
    pub window: usize,
}

impl SwinConfig {
    /// Swin-T (BiRefNet_lite).
    pub const TINY: SwinConfig = SwinConfig { embed: 96, depths: [2, 2, 6, 2], heads: [3, 6, 12, 24], window: 7 };
    /// Swin-L (BiRefNet).
    pub const LARGE: SwinConfig = SwinConfig { embed: 192, depths: [2, 2, 18, 2], heads: [6, 12, 24, 48], window: 12 };
}

struct SwinBlock {
    norm1: Norm,
    qkv: Linear,
    proj: Linear,
    /// relative Positions-Bias `[heads, N, N]`
    rel_bias: Vec<f32>,
    norm2: Norm,
    fc1: Linear,
    fc2: Linear,
    heads: usize,
}

struct SwinStage {
    blocks: Vec<SwinBlock>,
    down: Option<(Norm, Linear)>,
}

struct Swin {
    patch: Conv2d,
    patch_norm: Norm,
    stages: Vec<SwinStage>,
    out_norms: Vec<Norm>,
    cfg: SwinConfig,
}

/// Merkmale einer Stufe `[C, H, W]`.
#[derive(Clone, Debug)]
pub struct Feat {
    pub data: Vec<f32>,
    pub c: usize,
    pub h: usize,
    pub w: usize,
}

impl Swin {
    fn load(wt: &mut Weights, cfg: SwinConfig) -> Result<Swin> {
        let patch = wt.conv("bb.patch_embed.proj", 4, 0, 1)?;
        let patch_norm = wt.norm("bb.patch_embed.norm", 1e-5)?;
        let ws = cfg.window;
        let n = ws * ws;
        let mut stages = Vec::new();
        let mut out_norms = Vec::new();
        for s in 0..4 {
            let mut blocks = Vec::new();
            for b in 0..cfg.depths[s] {
                let p = format!("bb.layers.{s}.blocks.{b}");
                let table = wt.vec(&format!("{p}.attn.relative_position_bias_table"))?;
                let heads = cfg.heads[s];
                // Index selbst berechnen (wie im Original-Code), die gespeicherte Tabelle ist gleich.
                let _ = wt.opt(&format!("{p}.attn.relative_position_index"));
                let mut rel_bias = vec![0.0f32; heads * n * n];
                for i in 0..n {
                    for j in 0..n {
                        let (yi, xi) = (i / ws, i % ws);
                        let (yj, xj) = (j / ws, j % ws);
                        let dy = yi as isize - yj as isize + ws as isize - 1;
                        let dx = xi as isize - xj as isize + ws as isize - 1;
                        let idx = dy as usize * (2 * ws - 1) + dx as usize;
                        for h in 0..heads {
                            rel_bias[(h * n + i) * n + j] = table.get(idx * heads + h).copied().unwrap_or(0.0);
                        }
                    }
                }
                blocks.push(SwinBlock {
                    norm1: wt.norm(&format!("{p}.norm1"), 1e-5)?,
                    qkv: wt.linear(&format!("{p}.attn.qkv"))?,
                    proj: wt.linear(&format!("{p}.attn.proj"))?,
                    rel_bias,
                    norm2: wt.norm(&format!("{p}.norm2"), 1e-5)?,
                    fc1: wt.linear(&format!("{p}.mlp.fc1"))?,
                    fc2: wt.linear(&format!("{p}.mlp.fc2"))?,
                    heads,
                });
            }
            let down = if s < 3 { Some((wt.norm(&format!("bb.layers.{s}.downsample.norm"), 1e-5)?, wt.linear(&format!("bb.layers.{s}.downsample.reduction"))?)) } else { None };
            stages.push(SwinStage { blocks, down });
            out_norms.push(wt.norm(&format!("bb.norm{s}"), 1e-5)?);
        }
        Ok(Swin { patch, patch_norm, stages, out_norms, cfg })
    }

    /// `x[3, H, W]` (H, W Vielfache von 4) → vier Merkmalsstufen.
    fn forward(&self, x: &[f32], h: usize, w: usize, prog: &Progress, p0: f32, p1: f32) -> Result<Vec<Feat>> {
        let (pe, mut hh, mut ww) = self.patch.forward(x, h, w);
        let mut c = self.cfg.embed;
        let mut t = nn::transpose(&pe, c, hh * ww); // [HW, C]
        self.patch_norm.rows(&mut t);
        let mut outs = Vec::new();
        let total: usize = self.cfg.depths.iter().sum();
        let mut done = 0;
        let mut ts = std::time::Instant::now();
        for (s, st) in self.stages.iter().enumerate() {
            for (bi, blk) in st.blocks.iter().enumerate() {
                let shift = if bi % 2 == 1 { self.cfg.window / 2 } else { 0 };
                let nt = swin_block(blk, &t, hh, ww, c, self.cfg.window, shift);
                nn::give(std::mem::replace(&mut t, nt));
                done += 1;
                prog.step(p0, p1, done as f32 / total as f32)?;
            }
            nn::prof(&format!("  swin stufe {s} ({hh}x{ww}x{c})"), ts);
            ts = std::time::Instant::now();
            let mut o = nn::take_copy(&t);
            self.out_norms[s].rows(&mut o);
            outs.push(Feat { data: nn::transpose(&o, hh * ww, c), c, h: hh, w: ww });
            nn::give(o);
            if let Some((norm, red)) = &st.down {
                let (mut m, h2, w2) = patch_merge(&t, hh, ww, c);
                norm.rows(&mut m);
                nn::give(std::mem::replace(&mut t, red.forward(&m, h2 * w2)));
                nn::give(m);
                hh = h2;
                ww = w2;
                c *= 2;
            }
        }
        nn::give(t);
        nn::give(pe);
        Ok(outs)
    }
}

fn swin_block(b: &SwinBlock, x: &[f32], h: usize, w: usize, c: usize, ws: usize, shift: usize) -> Vec<f32> {
    use rayon::prelude::*;
    let tp = std::time::Instant::now();
    let hp = h.div_ceil(ws) * ws;
    let wp = w.div_ceil(ws) * ws;
    let (nwy, nwx) = (hp / ws, wp / ws);
    let nwin = nwy * nwx;
    let n = ws * ws;
    // Fenster einsammeln (zyklische Verschiebung, Rand mit Nullen) und dabei normieren
    let src: Vec<usize> = (0..nwin * n)
        .map(|i| {
            let (win, t) = (i / n, i % n);
            let (wy, wx, ty, tx) = (win / nwx, win % nwx, t / ws, t % ws);
            let yy = (wy * ws + ty + shift) % hp;
            let xx = (wx * ws + tx + shift) % wp;
            if yy < h && xx < w { yy * w + xx } else { usize::MAX }
        })
        .collect();
    let mut xw = nn::take(nwin * n * c);
    xw.par_chunks_mut(c * 64).enumerate().for_each(|(ci, ch)| {
        for (j, row) in ch.chunks_mut(c).enumerate() {
            let s = src[ci * 64 + j];
            if s != usize::MAX {
                row.copy_from_slice(&x[s * c..(s + 1) * c]);
                nn::layer_norm_rows_seq(row, c, &b.norm1.w, &b.norm1.b, b.norm1.eps);
            } else {
                row.fill(0.0);
            }
        }
    });
    let tp = nn::prof("      norm+gather", tp);
    let qkv = b.qkv.forward(&xw, nwin * n);
    nn::give(xw);
    let tp = nn::prof("      qkv", tp);
    // Maske für verschobene Fenster: Regionen im verschobenen Raster
    let region = |y: usize, lim: usize| -> u8 {
        if y < lim - ws {
            0
        } else if y < lim - shift {
            1
        } else {
            2
        }
    };
    let labels: Vec<u8> = if shift > 0 {
        (0..nwin * n)
            .map(|i| {
                let (win, t) = (i / n, i % n);
                let (yy, xx) = ((win / nwx) * ws + t / ws, (win % nwx) * ws + t % ws);
                region(yy, hp) * 3 + region(xx, wp)
            })
            .collect()
    } else {
        Vec::new()
    };
    let heads = b.heads;
    let d = c / heads;
    let bias = |g: usize, hh: usize, q0: usize, s: &mut [f32]| {
        let nn_ = s.len() / n;
        let rb = &b.rel_bias[(hh * n + q0) * n..(hh * n + q0 + nn_) * n];
        for (a, r) in s.iter_mut().zip(rb) {
            *a += *r;
        }
        if shift > 0 {
            let lab = &labels[g * n..(g + 1) * n];
            for i in 0..nn_ {
                let li = lab[q0 + i];
                for (j, lj) in lab.iter().enumerate() {
                    if li != *lj {
                        s[i * n + j] -= 100.0;
                    }
                }
            }
        }
    };
    let pk = |off: usize| nn::Packed { buf: &qkv, stride: 3 * c, off };
    let att = nn::attention_packed(pk(0), pk(c), pk(2 * c), nwin, n, n, heads, d, (d as f32).powf(-0.5), Some(&bias));
    nn::give(qkv);
    let tp = nn::prof("      attention", tp);
    let out = b.proj.forward(&att, nwin * n);
    nn::give(att);
    let tp = nn::prof("      proj", tp);
    // zurückverteilen + Restverbindung
    let mut inv = vec![usize::MAX; h * w];
    for (i, s) in src.iter().enumerate() {
        if *s != usize::MAX {
            inv[*s] = i;
        }
    }
    let mut y = nn::take_copy(x);
    y.par_chunks_mut(c).zip(inv.par_iter()).for_each(|(row, i)| {
        if *i != usize::MAX {
            for (a, bb) in row.iter_mut().zip(&out[i * c..(i + 1) * c]) {
                *a += *bb;
            }
        }
    });
    nn::give(out);
    let tp = nn::prof("      scatter", tp);
    // MLP
    let mut yn = nn::take_copy(&y);
    b.norm2.rows(&mut yn);
    let tp = nn::prof("      norm2", tp);
    let mut hdn = b.fc1.forward(&yn, h * w);
    nn::give(yn);
    let tp = nn::prof("      fc1", tp);
    nn::gelu_inplace(&mut hdn);
    let tp = nn::prof("      gelu", tp);
    let o = b.fc2.forward(&hdn, h * w);
    nn::give(hdn);
    let tp = nn::prof("      fc2", tp);
    nn::add_inplace(&mut y, &o);
    nn::give(o);
    nn::prof("      add", tp);
    y
}

fn patch_merge(x: &[f32], h: usize, w: usize, c: usize) -> (Vec<f32>, usize, usize) {
    let (h2, w2) = (h.div_ceil(2), w.div_ceil(2));
    let mut o = vec![0.0f32; h2 * w2 * 4 * c];
    use rayon::prelude::*;
    o.par_chunks_mut(4 * c).enumerate().for_each(|(i, row)| {
        let (y, xx) = (i / w2, i % w2);
        // Reihenfolge x0 (0,0), x1 (1,0), x2 (0,1), x3 (1,1) – (dy, dx)
        for (q, (dy, dx)) in [(0, 0), (1, 0), (0, 1), (1, 1)].iter().enumerate() {
            let (sy, sx) = (2 * y + dy, 2 * xx + dx);
            if sy < h && sx < w {
                row[q * c..(q + 1) * c].copy_from_slice(&x[(sy * w + sx) * c..(sy * w + sx + 1) * c]);
            }
        }
    });
    (o, h2, w2)
}

// ------------------------------------------------------------------ Decoder

struct DeformConv {
    /// Offset- und Modulator-Faltung zusammengelegt: Ausgänge `2·kk` Offsets, dann `kk` Masken.
    offmod: Conv2d,
    regular: Conv2d,
}

impl DeformConv {
    fn load(wt: &mut Weights, p: &str, bn: &str, k: usize) -> Result<DeformConv> {
        let pad = k / 2;
        let off = wt.conv(&format!("{p}.offset_conv"), 1, pad, 1)?;
        let m = wt.conv(&format!("{p}.modulator_conv"), 1, pad, 1)?;
        let mut w = off.w;
        w.extend(m.w);
        let mut b = off.b.unwrap_or_else(|| vec![0.0; off.cout]);
        b.extend(m.b.unwrap_or_else(|| vec![0.0; m.cout]));
        let offmod = Conv2d { w, b: Some(b), cout: off.cout + m.cout, cin: off.cin, k, stride: 1, pad, dil: 1 };
        Ok(DeformConv { offmod, regular: wt.conv_bn(&format!("{p}.regular_conv"), bn, 1, pad, 1)? })
    }
    fn forward(&self, x: &[f32], h: usize, w: usize) -> Vec<f32> {
        let t = std::time::Instant::now();
        let kk = self.regular.k * self.regular.k;
        let (mut om, _, _) = self.offmod.forward(x, h, w);
        let hw = h * w;
        nn::map_inplace(&mut om[2 * kk * hw..], |v| 2.0 * nn::sigmoid(v));
        let t = nn::prof("    offset+mod", t);
        let (mut o, _, _) = nn::deform_conv2d(x, h, w, &self.regular, &om[..2 * kk * hw], &om[2 * kk * hw..]);
        nn::give(om);
        nn::relu_inplace(&mut o);
        nn::prof("    deform", t);
        o
    }
}

struct AsppDeformable {
    aspp1: DeformConv,
    deforms: Vec<DeformConv>,
    gap: Conv2d,
    /// `conv1` (1×1 über 5·256 Kanäle) zerlegt je Zweig: `[cout, 256]`.
    conv1_parts: Vec<Vec<f32>>,
    conv1_bias: Vec<f32>,
    cout: usize,
}

impl AsppDeformable {
    fn load(wt: &mut Weights, p: &str) -> Result<AsppDeformable> {
        let aspp1 = DeformConv::load(wt, &format!("{p}.aspp1.atrous_conv"), &format!("{p}.aspp1.bn"), 1)?;
        let mut deforms = Vec::new();
        for (i, k) in [1usize, 3, 7].iter().enumerate() {
            deforms.push(DeformConv::load(wt, &format!("{p}.aspp_deforms.{i}.atrous_conv"), &format!("{p}.aspp_deforms.{i}.bn"), *k)?);
        }
        let conv1 = wt.conv_bn(&format!("{p}.conv1"), &format!("{p}.bn1"), 1, 0, 1)?;
        let parts = 5;
        let pc = conv1.cin / parts;
        let conv1_parts = (0..parts)
            .map(|b| {
                let mut m = vec![0.0f32; conv1.cout * pc];
                for o in 0..conv1.cout {
                    m[o * pc..(o + 1) * pc].copy_from_slice(&conv1.w[o * conv1.cin + b * pc..o * conv1.cin + (b + 1) * pc]);
                }
                m
            })
            .collect();
        Ok(AsppDeformable {
            aspp1,
            deforms,
            gap: wt.conv_bn(&format!("{p}.global_avg_pool.1"), &format!("{p}.global_avg_pool.2"), 1, 0, 1)?,
            conv1_bias: conv1.b.clone().unwrap_or_else(|| vec![0.0; conv1.cout]),
            cout: conv1.cout,
            conv1_parts,
        })
    }
    fn forward(&self, x: &[f32], c: usize, h: usize, w: usize) -> Vec<f32> {
        let hw = h * w;
        let pc = self.gap.cout;
        // globaler Mittelwert → konstanter Beitrag je Ausgangskanal
        let mean: Vec<f32> = (0..c).map(|k| x[k * hw..(k + 1) * hw].iter().map(|v| *v as f64).sum::<f64>() as f32 / hw as f32).collect();
        let (mut g, _, _) = self.gap.forward(&mean, 1, 1);
        nn::relu_inplace(&mut g);
        let mut bias = self.conv1_bias.clone();
        for (o, b) in bias.iter_mut().enumerate() {
            *b += self.conv1_parts[4][o * pc..(o + 1) * pc].iter().zip(&g).map(|(a, b)| a * b).sum::<f32>();
        }
        // Zweige nacheinander: Beitrag jeweils direkt aufsummieren (kein Verketten)
        let mut out = nn::take_zeroed(self.cout * hw);
        for (bi, d) in std::iter::once(&self.aspp1).chain(self.deforms.iter()).enumerate() {
            let y = d.forward(x, h, w);
            nn::gemm(&self.conv1_parts[bi], self.cout, pc, &y, hw, false, &mut out, true, true);
            nn::give(y);
        }
        nn::add_channel_bias(&mut out, &bias);
        nn::relu_inplace(&mut out);
        out
    }
}

struct DecBlk {
    conv_in: Conv2d,
    att: AsppDeformable,
    conv_out: Conv2d,
}

impl DecBlk {
    fn load(wt: &mut Weights, p: &str) -> Result<DecBlk> {
        Ok(DecBlk {
            conv_in: wt.conv_bn(&format!("{p}.conv_in"), &format!("{p}.bn_in"), 1, 1, 1)?,
            att: AsppDeformable::load(wt, &format!("{p}.dec_att"))?,
            conv_out: wt.conv_bn(&format!("{p}.conv_out"), &format!("{p}.bn_out"), 1, 1, 1)?,
        })
    }
    fn forward(&self, x: &[f32], h: usize, w: usize) -> Vec<f32> {
        let t = std::time::Instant::now();
        let (mut a, _, _) = self.conv_in.forward(x, h, w);
        nn::relu_inplace(&mut a);
        let t = nn::prof("  conv_in", t);
        let b = self.att.forward(&a, self.conv_in.cout, h, w);
        nn::give(a);
        let t = nn::prof("  aspp", t);
        let o = self.conv_out.forward(&b, h, w).0;
        nn::give(b);
        nn::prof("  conv_out", t);
        o
    }
}

struct SimpleConvs {
    conv1: Conv2d,
    conv_out: Conv2d,
}

impl SimpleConvs {
    fn load(wt: &mut Weights, p: &str) -> Result<SimpleConvs> {
        Ok(SimpleConvs { conv1: wt.conv(&format!("{p}.conv1"), 1, 1, 1)?, conv_out: wt.conv(&format!("{p}.conv_out"), 1, 1, 1)? })
    }
    fn forward(&self, x: &[f32], h: usize, w: usize) -> Vec<f32> {
        let (a, _, _) = self.conv1.forward(x, h, w);
        self.conv_out.forward(&a, h, w).0
    }
}

struct Gdt {
    conv: Conv2d,
    attn: Conv2d,
}

impl Gdt {
    fn load(wt: &mut Weights, i: usize) -> Result<Gdt> {
        // Vorhersage-Zweig (`gdt_convs_pred_*`) wird nur im Training gebraucht.
        let _ = wt.opt(&format!("decoder.gdt_convs_pred_{i}.0.weight"));
        Ok(Gdt {
            conv: wt.conv_bn(&format!("decoder.gdt_convs_{i}.0"), &format!("decoder.gdt_convs_{i}.1"), 1, 1, 1)?,
            attn: wt.conv(&format!("decoder.gdt_convs_attn_{i}.0"), 1, 0, 1)?,
        })
    }
    /// `p *= sigmoid(attn(relu(conv(p))))`
    fn apply(&self, p: &mut [f32], c: usize, h: usize, w: usize) {
        let (mut g, _, _) = self.conv.forward(p, h, w);
        nn::relu_inplace(&mut g);
        let (mut a, _, _) = self.attn.forward(&g, h, w);
        nn::give(g);
        nn::map_inplace(&mut a, nn::sigmoid);
        let hw = h * w;
        for k in 0..c {
            nn::mul_inplace(&mut p[k * hw..(k + 1) * hw], &a);
        }
    }
}

/// Ein geladenes BiRefNet.
pub struct BiRefNet {
    swin: Swin,
    squeeze: DecBlk,
    ipt: [SimpleConvs; 5],
    dec: [DecBlk; 4],
    lateral: [Conv2d; 3],
    gdt: [Gdt; 3],
    /// Letzte 1×1-Faltung, auf den Decoder-Zweig angewandt (vor dem Hochskalieren, linear).
    final_a: Vec<f32>,
    /// ipt_blk1.conv_out mit dem Bildfeld-Teil der letzten Faltung zusammengelegt (64 → 1).
    final_conv: Conv2d,
}

/// `image2patches(x, grid)` + Interpolation auf dieselbe Größe: `[3, S, S]` → `[3·g·g, S/g, S/g]`.
fn image_patches(x: &[f32], s: usize, size: usize) -> Vec<f32> {
    let g = s / size;
    let mut o = vec![0.0f32; 3 * g * g * size * size];
    use rayon::prelude::*;
    o.par_chunks_mut(size * size).enumerate().for_each(|(ch, plane)| {
        let c = ch / (g * g);
        let gy = (ch / g) % g;
        let gx = ch % g;
        for y in 0..size {
            let src = &x[c * s * s + (gy * size + y) * s + gx * size..][..size];
            plane[y * size..(y + 1) * size].copy_from_slice(src);
        }
    });
    o
}

impl BiRefNet {
    /// Aus Gewichten aufbauen.
    pub fn load(mut wt: Weights) -> Result<BiRefNet> {
        let cfg = if wt.get("bb.patch_embed.proj.weight")?.shape.first() == Some(&192) { SwinConfig::LARGE } else { SwinConfig::TINY };
        let swin = Swin::load(&mut wt, cfg)?;
        let squeeze = DecBlk::load(&mut wt, "squeeze_module.0")?;
        let ipt = [
            SimpleConvs::load(&mut wt, "decoder.ipt_blk1")?,
            SimpleConvs::load(&mut wt, "decoder.ipt_blk2")?,
            SimpleConvs::load(&mut wt, "decoder.ipt_blk3")?,
            SimpleConvs::load(&mut wt, "decoder.ipt_blk4")?,
            SimpleConvs::load(&mut wt, "decoder.ipt_blk5")?,
        ];
        let dec = [
            DecBlk::load(&mut wt, "decoder.decoder_block1")?,
            DecBlk::load(&mut wt, "decoder.decoder_block2")?,
            DecBlk::load(&mut wt, "decoder.decoder_block3")?,
            DecBlk::load(&mut wt, "decoder.decoder_block4")?,
        ];
        let lateral = [wt.conv("decoder.lateral_block2.conv", 1, 0, 1)?, wt.conv("decoder.lateral_block3.conv", 1, 0, 1)?, wt.conv("decoder.lateral_block4.conv", 1, 0, 1)?];
        let gdt = [Gdt::load(&mut wt, 2)?, Gdt::load(&mut wt, 3)?, Gdt::load(&mut wt, 4)?];
        let conv_out1 = wt.conv("decoder.conv_out1.0", 1, 0, 1)?;
        // conv_out1([hoch(p1); ipt1(x)]) ist linear: Teil A vor dem Hochskalieren anwenden, Teil B
        // in ipt_blk1.conv_out hineinfalten.
        let cp1 = dec[0].conv_out.cout;
        let final_a = conv_out1.w[..cp1].to_vec();
        let wb = &conv_out1.w[cp1..];
        let co = &ipt[0].conv_out;
        let per = co.cin * co.k * co.k;
        let mut fw = vec![0.0f32; per];
        let mut fb = conv_out1.b.as_ref().and_then(|b| b.first().copied()).unwrap_or(0.0);
        for (o, wo) in wb.iter().enumerate() {
            for (a, b) in fw.iter_mut().zip(&co.w[o * per..(o + 1) * per]) {
                *a += wo * b;
            }
            fb += wo * co.b.as_ref().map(|b| b[o]).unwrap_or(0.0);
        }
        let final_conv = Conv2d { w: fw, b: Some(vec![fb]), cout: 1, cin: co.cin, k: co.k, stride: 1, pad: co.pad, dil: 1 };
        Ok(BiRefNet { swin, squeeze, ipt, dec, lateral, gdt, final_a, final_conv })
    }

    /// Größe des Backbones.
    pub fn swin_config(&self) -> SwinConfig {
        self.swin.cfg
    }

    /// Vorwärtsrechnung: `x[3, s, s]` (normalisiert, `s` Vielfaches von 32) → Logits `[s, s]`.
    pub fn forward(&self, x: &[f32], s: usize, prog: &Progress) -> Result<Vec<f32>> {
        if !s.is_multiple_of(32) || x.len() != 3 * s * s {
            return Err(AiError::Input(format!("BiRefNet braucht ein Quadrat mit Kantenlänge Vielfaches von 32 (erhalten {s})")));
        }
        // Encoder: volle und halbe Auflösung
        let t0 = std::time::Instant::now();
        let f_full = self.swin.forward(x, s, s, prog, 0.0, 0.45)?;
        let t0 = nn::prof("swin voll", t0);
        let half = nn::resize_bilinear(x, 3, s, s, s / 2, s / 2, true);
        let f_half = self.swin.forward(&half, s / 2, s / 2, prog, 0.45, 0.6)?;
        nn::give(half);
        let t0 = nn::prof("swin halb", t0);
        let mut feats: Vec<Feat> = Vec::new();
        for (a, b) in f_full.into_iter().zip(f_half) {
            let up = nn::resize_bilinear(&b.data, b.c, b.h, b.w, a.h, a.w, true);
            let data = nn::concat_channels(&[&a.data, &up]);
            nn::give(up);
            nn::give(a.data);
            nn::give(b.data);
            feats.push(Feat { data, c: a.c * 2, h: a.h, w: a.w });
        }
        prog.step(0.6, 0.62, 1.0)?;
        // Kontext: x1, x2, x3 auf die Größe von x4 + x4
        let (h4, w4) = (feats[3].h, feats[3].w);
        let mut parts: Vec<Vec<f32>> = Vec::new();
        for f in feats.iter().take(3) {
            parts.push(nn::resize_bilinear(&f.data, f.c, f.h, f.w, h4, w4, true));
        }
        let x4 = nn::concat_channels(&[&parts[0], &parts[1], &parts[2], &feats[3].data]);
        parts.into_iter().for_each(nn::give);
        let t0 = nn::prof("kontext", t0);
        let sq = self.squeeze.forward(&x4, h4, w4);
        nn::give(x4);
        let t0 = nn::prof("squeeze", t0);
        prog.step(0.62, 0.66, 1.0)?;

        // Decoder
        let ipt = |i: usize, size: usize| -> Vec<f32> {
            let p = image_patches(x, s, size);
            let o = self.ipt[i].forward(&p, size, size);
            nn::give(p);
            o
        };
        // eine Decoder-Stufe: (hoch(p) + lateral(f)) ⧺ Bildfelder → Block → Gradienten-Aufmerksamkeit
        let stage = |inp: Vec<f32>, blk: usize, gdt: Option<usize>, h: usize, w: usize, ipt_i: usize| -> Vec<f32> {
            let pi = ipt(ipt_i, h);
            let cat = nn::concat_channels(&[&inp, &pi]);
            nn::give(inp);
            nn::give(pi);
            let mut p = self.dec[blk].forward(&cat, h, w);
            nn::give(cat);
            if let Some(g) = gdt {
                self.gdt[g].apply(&mut p, self.dec[blk].conv_out.cout, h, w);
            }
            p
        };
        let up_lat = |p: Vec<f32>, c: usize, h: usize, w: usize, f: &Feat, lat: usize| -> Vec<f32> {
            let mut u = nn::resize_bilinear(&p, c, h, w, f.h, f.w, true);
            nn::give(p);
            let l = self.lateral[lat].forward(&f.data, f.h, f.w).0;
            nn::add_inplace(&mut u, &l);
            nn::give(l);
            u
        };
        let p4 = stage(sq, 3, Some(2), h4, w4, 4);
        let cp4 = self.dec[3].conv_out.cout;
        prog.step(0.66, 0.7, 1.0)?;
        let t0 = nn::prof("dec4", t0);

        let f3 = &feats[2];
        let p3 = stage(up_lat(p4, cp4, h4, w4, f3, 2), 2, Some(1), f3.h, f3.w, 3);
        let cp3 = self.dec[2].conv_out.cout;
        prog.step(0.7, 0.75, 1.0)?;
        let t0 = nn::prof("dec3", t0);

        let f2 = &feats[1];
        let p2 = stage(up_lat(p3, cp3, f3.h, f3.w, f2, 1), 1, Some(0), f2.h, f2.w, 2);
        let cp2 = self.dec[1].conv_out.cout;
        prog.step(0.75, 0.82, 1.0)?;
        let t0 = nn::prof("dec2", t0);

        let f1 = &feats[0];
        let p1 = stage(up_lat(p2, cp2, f2.h, f2.w, f1, 0), 0, None, f1.h, f1.w, 1);
        let cp1 = self.dec[0].conv_out.cout;
        prog.step(0.82, 0.95, 1.0)?;
        let t0 = nn::prof("dec1", t0);

        let hw1 = f1.h * f1.w;
        let mut r = vec![0.0f32; hw1];
        nn::gemm(&self.final_a, 1, cp1, &p1, hw1, false, &mut r, false, true);
        nn::give(p1);
        let up = nn::resize_bilinear(&r, 1, f1.h, f1.w, s, s, true);
        let (a1, _, _) = self.ipt[0].conv1.forward(x, s, s);
        let (mut out, _, _) = self.final_conv.forward(&a1, s, s);
        nn::give(a1);
        nn::add_inplace(&mut out, &up);
        nn::give(up);
        feats.into_iter().for_each(|f| nn::give(f.data));
        nn::prof("ende", t0);
        prog.step(0.95, 1.0, 1.0)?;
        Ok(out)
    }
}
