//! Incremental retouch strokes (FB-39): Blur, Sharpen, Dodge, Burn and Smudge as a stroke that
//! grows point by point. `bitmap.retouch` feeds all points at once; the live preview of the
//! retouch tools feeds each new pointer position and gets back the rectangle of pixels that can
//! have changed, so a pointer move costs the new dabs only, not the whole stroke from its first dab
//! again (which made long strokes and large tips lag: every move recomputed and re-tiled the full
//! stroke). The result of every pixel depends only on the point sequence, not on how it was cut
//! into pushes, so the last preview equals the committed result.
//!
//! The models are the measured ones of `tooling.rs` (see there): Dodge/Burn expose each pixel once
//! by the coverage of the whole stroke; Blur/Sharpen work in up to three passes over the stroke
//! area, each pass reading the previous one (here kept as three images, a new dab recomputes pass
//! 1 in its area, pass 2 one pixel and pass 3 two pixels around it); Smudge drags the content dab
//! by dab, each dab reading the state after the previous one. Work buffers cover only the part of
//! the bitmap the stroke reaches (growing geometrically), not the whole bitmap.
//!
//! A stroke that was paused and resumed (FB-31, `breaks`) consists of parts: a point whose index
//! is a break starts a new part, whose dabs are not joined to the previous part. Dodge/Burn and
//! Blur/Sharpen still work the whole stroke once (the coverage and the mouse points of all parts
//! together, so an overlap is not worked twice); Smudge starts every part afresh at its press (no
//! drag from the previous part, "Smudge color" from full strength, the growth decided again).

use super::bitmap::Dab;
use super::tooling::{
    BLUR_GAIN, OUTSIDE, Retouch, SHARPEN_DAB_GAIN, in_rect, premul, tone_curve, unpremul,
};
use fp_color::Rgba8;
use fp_geom::{IRect, Point};
use fp_raster::{Mask, RgbaImage};
use std::sync::Arc;

/// Far-away limit for buffers that may grow past the bitmap (smudge).
const SANE: IRect = IRect {
    left: -1_000_000,
    top: -1_000_000,
    right: 1_000_000,
    bottom: 1_000_000,
};

/// Outside colour of the blur as stored pixel (transparent white).
const OUTSIDE_PX: [u8; 4] = [255, 255, 255, 0];

/// Values over a rectangle that grows on demand (new cells from an init function).
#[derive(Clone, Debug)]
struct Grow<T: Copy> {
    r: IRect,
    w: usize,
    v: Vec<T>,
}

impl<T: Copy> Grow<T> {
    fn empty() -> Grow<T> {
        Grow {
            r: IRect::default(),
            w: 0,
            v: Vec::new(),
        }
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        in_rect(&self.r, x, y)
            .then(|| (y - self.r.top) as usize * self.w + (x - self.r.left) as usize)
    }

    #[inline]
    fn get(&self, x: i32, y: i32) -> Option<T> {
        self.idx(x, y).map(|i| self.v[i])
    }

    /// Makes the buffer cover `need ∩ limit`; `next` = the rectangle to allocate when it must grow
    /// (computed once per stroke state, so that all buffers of a stroke keep the same rectangle).
    fn ensure(&mut self, next: IRect, init: impl Fn(i32, i32) -> T) {
        if next == self.r {
            return;
        }
        let w = next.width() as usize;
        let h = next.height() as usize;
        let mut v = Vec::with_capacity(w * h);
        for y in next.top..next.bottom {
            for x in next.left..next.right {
                v.push(match self.idx(x, y) {
                    Some(i) => self.v[i],
                    None => init(x, y),
                });
            }
        }
        *self = Grow { r: next, w, v };
    }
}

/// Buffers up to this many pixels are allocated once for the whole bitmap at the first dab, so a
/// drag never pauses to reallocate them; larger bitmaps grow the buffers with the stroke.
const PREALLOC: u64 = 4_000_000;

/// Rectangle a set of buffers must grow to so that it covers `need` (clipped to `limit`): at first
/// `first` (the bitmap) when it is small enough, otherwise with a margin of half its size (at least
/// 64 px), so that a long stroke reallocates only a few times.
fn grown(cur: IRect, need: IRect, limit: IRect, first: IRect) -> IRect {
    let need = need.intersect(&limit);
    if need.is_empty() || (cur.intersect(&need) == need && !cur.is_empty()) {
        return cur;
    }
    if cur.is_empty() {
        let f = first.intersect(&limit).union(&need);
        if f.area() <= PREALLOC {
            return f;
        }
    }
    let u = if cur.is_empty() {
        need
    } else {
        cur.union(&need)
    };
    let mx = (u.width() / 2).max(64) as i32;
    let my = (u.height() / 2).max(64) as i32;
    let next = IRect::new(u.left - mx, u.top - my, u.right + mx, u.bottom + my).intersect(&limit);
    if cur.is_empty() {
        next
    } else {
        next.union(&cur)
    }
}

/// Dab positions along the points, one push at a time; the same positions as
/// [`super::bitmap::dab_points`] on the whole sequence.
#[derive(Clone, Debug)]
struct DabWalker {
    last: Option<Point>,
    carry: f32,
    step: f32,
}

impl DabWalker {
    fn new(step: f32) -> DabWalker {
        DabWalker {
            last: None,
            carry: 0.0,
            step: step.max(0.25),
        }
    }

    fn push(&mut self, b: Point, out: &mut Vec<Point>) {
        let Some(a) = self.last.replace(b) else {
            out.push(b);
            return;
        };
        let len = (b.x - a.x).hypot(b.y - a.y);
        if len <= 0.0 {
            return;
        }
        let mut t = self.step - self.carry;
        while t <= len {
            let f = t / len;
            out.push(Point::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f));
            t += self.step;
        }
        self.carry = (self.carry + len) % self.step;
    }
}

/// Dodge/Burn state: product of (1 − coverage) per pixel and the exposed pixels.
#[derive(Clone, Debug)]
struct Cover {
    keep: Grow<f32>,
    out: Grow<[u8; 4]>,
}

/// Blur/Sharpen state.
#[derive(Clone, Debug)]
struct Passes {
    /// Bitmap plus the 2 px the blur may grow (sharpen: the bitmap).
    ext: IRect,
    /// Union of the dab bounds (the stroke area).
    reach: IRect,
    keep: Grow<f32>,
    /// Mouse points within reach of each pixel (at most 3).
    hits: Grow<u8>,
    /// Pass results 1..3.
    pass: [Grow<[u8; 4]>; 3],
    events: usize,
    last_event: Option<Point>,
}

/// Smudge state.
#[derive(Clone, Debug)]
struct Smear {
    /// Not decided until the first point of the part (grows past the edge or not).
    grow: Option<bool>,
    /// Some part grew the bitmap past its edge (the finished bitmap may be larger).
    grew: bool,
    reach: IRect,
    buf: Grow<[f32; 4]>,
    touched: Grow<bool>,
    prev: Option<Point>,
    color_w: f32,
}

#[derive(Clone, Debug)]
enum State {
    Cover(Cover),
    Passes(Box<Passes>),
    Smear(Box<Smear>),
}

/// A running Blur/Sharpen/Dodge/Burn/Smudge stroke on a bitmap.
#[derive(Clone, Debug)]
pub(crate) struct RetouchStroke {
    prm: Retouch,
    base: Arc<RgbaImage>,
    sel: Option<Arc<Mask>>,
    /// Source page of "Entire document" (smudge).
    page: Option<Arc<RgbaImage>>,
    dab: Dab,
    walker: DabWalker,
    state: State,
    /// Points fed so far (the index of the next point in the whole stroke).
    fed: usize,
    /// Index of the first point of the running part (FB-31).
    part_start: usize,
}

impl RetouchStroke {
    /// New stroke on `base` (the whole target bitmap); `None` for a mode this engine does not run
    /// (stamp, replace colour, red eye).
    pub(crate) fn new(
        base: Arc<RgbaImage>,
        prm: Retouch,
        sel: Option<Arc<Mask>>,
        page: Option<Arc<RgbaImage>>,
    ) -> Option<RetouchStroke> {
        let orig = base.bounds();
        let state = match prm.mode.as_str() {
            "dodge" | "burn" => State::Cover(Cover {
                keep: Grow::empty(),
                out: Grow::empty(),
            }),
            "blur" | "sharpen" => {
                let ext = if prm.mode == "blur" && !orig.is_empty() {
                    orig.inflate(2, 2)
                } else {
                    orig
                };
                State::Passes(Box::new(Passes {
                    ext,
                    reach: IRect::default(),
                    keep: Grow::empty(),
                    hits: Grow::empty(),
                    pass: [Grow::empty(), Grow::empty(), Grow::empty()],
                    events: 0,
                    last_event: None,
                }))
            }
            "smudge" => State::Smear(Box::new(Smear {
                grow: None,
                grew: false,
                reach: IRect::default(),
                buf: Grow::empty(),
                touched: Grow::empty(),
                prev: None,
                color_w: 1.0,
            })),
            _ => return None,
        };
        let dab = Dab::stroke(prm.size, prm.round, prm.soft * 100.0);
        Some(RetouchStroke {
            prm,
            base,
            sel,
            page,
            dab,
            walker: DabWalker::new(1.0),
            state,
            fed: 0,
            part_start: 0,
        })
    }

    /// The bitmap the stroke started on.
    pub(crate) fn base(&self) -> &Arc<RgbaImage> {
        &self.base
    }

    /// Number of points fed so far.
    pub(crate) fn fed(&self) -> usize {
        self.fed
    }

    /// Feeds the points of the whole stroke `all` that were not fed yet, with the breaks of a
    /// resumed stroke (FB-31: indices into `all`; the same list `bitmap.retouch` takes as
    /// `breaks`), so the live stroke and the command split the stroke into the same parts. Returns
    /// the rectangle of pixels that may have changed.
    pub(crate) fn feed(&mut self, all: &[Point], breaks: &[usize]) -> IRect {
        if self.prm.breaks != breaks {
            self.prm.breaks = breaks.to_vec();
        }
        let from = self.fed.min(all.len());
        self.push(&all[from..])
    }

    /// Appends points (continuing the points fed so far); a point whose index in the whole stroke
    /// is one of `prm.breaks` starts a new part (FB-31; index 0 and repeated indices are no break,
    /// as in `paint::split_segments` for an increasing list). Returns the rectangle of pixels that
    /// may have changed.
    pub(crate) fn push(&mut self, pts: &[Point]) -> IRect {
        let mut dirty = IRect::default();
        let mut from = 0;
        for i in 0..pts.len() {
            let g = self.fed + i;
            if g > self.part_start && self.prm.breaks.contains(&g) {
                if from < i {
                    dirty = dirty.union(&self.push_part(&pts[from..i]));
                }
                self.resume(g);
                from = i;
            }
        }
        if from < pts.len() {
            dirty = dirty.union(&self.push_part(&pts[from..]));
        }
        self.fed += pts.len();
        dirty
    }

    /// Starts a new part at the point with index `at` (FB-31): its dabs are not joined to the
    /// previous part; the smudge starts afresh (no drag from the previous part, "Smudge color" at
    /// full strength, growth decided by the new press point).
    fn resume(&mut self, at: usize) {
        self.part_start = at;
        self.walker = DabWalker::new(1.0);
        if let State::Smear(s) = &mut self.state {
            s.grow = None;
            s.prev = None;
            s.color_w = 1.0;
        }
    }

    /// Appends points of the running part.
    fn push_part(&mut self, pts: &[Point]) -> IRect {
        let mut stamps = Vec::new();
        for p in pts {
            self.walker.push(*p, &mut stamps);
        }
        let orig = self.base.bounds();
        let sel = self.sel.as_deref();
        let prm = &self.prm;
        let dab = &self.dab;
        let base = &self.base;
        match &mut self.state {
            State::Cover(c) => push_cover(c, base, dab, prm, sel, &stamps),
            State::Passes(p) => push_passes(p, base, dab, prm, sel, pts, &stamps),
            State::Smear(s) => {
                if s.grow.is_none()
                    && let Some(q) = pts.first()
                {
                    // FB-15/FB-40: without "Confine to boundaries" the smear runs past the edge
                    // and grows the bitmap wherever it carries colour, also when the stroke
                    // started outside (dragging transparency in alone grows nothing); with
                    // "Entire document" only a stroke that starts on the bitmap grows it (the
                    // page would otherwise be smeared into new pixels far from the bitmap)
                    let inside = in_rect(&orig, q.x.floor() as i32, q.y.floor() as i32);
                    let grow = !prm.confine && !orig.is_empty() && (self.page.is_none() || inside);
                    s.grow = Some(grow);
                    s.grew |= grow;
                }
                push_smear(s, base, dab, prm, sel, self.page.as_deref(), &stamps)
            }
        }
    }

    /// Pixel of the stroke so far (outside the bitmap and its growth transparent).
    pub(crate) fn pixel(&self, x: i32, y: i32) -> [u8; 4] {
        let own = || self.base.get(x, y).unwrap_or([0; 4]);
        match &self.state {
            State::Cover(c) => c.out.get(x, y).unwrap_or_else(own),
            State::Passes(p) => p.pass[2].get(x, y).unwrap_or_else(own),
            State::Smear(s) => match (s.touched.get(x, y), s.buf.get(x, y)) {
                (Some(true), Some(v)) => v.map(|c| c as u8),
                _ => own(),
            },
        }
    }

    /// The finished bitmap (what `bitmap.retouch` stores).
    pub(crate) fn finish(&self) -> RgbaImage {
        let orig = self.base.bounds();
        match &self.state {
            State::Cover(c) => {
                let mut img = (*self.base).clone();
                copy_px(&mut img, &c.out, orig);
                img
            }
            State::Passes(p) => {
                let blur = self.prm.mode == "blur";
                let mut img = (*self.base).clone();
                if blur && !orig.is_empty() {
                    let ext = orig.union(&p.reach.intersect(&p.ext));
                    if ext != orig
                        && let Ok(mut e) = img.extended_to(ext)
                    {
                        for y in ext.top..ext.bottom {
                            for x in ext.left..ext.right {
                                if !in_rect(&orig, x, y) {
                                    e.set_pixel(x, y, Rgba8::new(255, 255, 255, 0));
                                }
                            }
                        }
                        img = e;
                    }
                }
                let b = img.bounds();
                copy_px(&mut img, &p.pass[2], b);
                if blur && !orig.is_empty() {
                    // keep only edges that really got coverage
                    let keep = orig.union(&img.alpha_bounds());
                    if keep != img.bounds() {
                        img = img.cropped(keep);
                    }
                }
                img
            }
            State::Smear(s) => {
                let mut img = (*self.base).clone();
                let work = s.reach.inflate(2, 2);
                if s.grew
                    && !orig.is_empty()
                    && !work.is_empty()
                    && let Ok(e) = img.extended_to(work)
                {
                    img = e;
                }
                let r = s.touched.r.intersect(&img.bounds());
                for y in r.top..r.bottom {
                    for x in r.left..r.right {
                        if s.touched.get(x, y) == Some(true)
                            && let Some(v) = s.buf.get(x, y)
                        {
                            let p = v.map(|c| c as u8);
                            img.set_pixel(x, y, Rgba8::new(p[0], p[1], p[2], p[3]));
                        }
                    }
                }
                if img.bounds() != orig {
                    // keep only the grown pixels that got coverage (the bitmap ends where the
                    // smear fades out)
                    let keep = orig.union(&img.alpha_bounds());
                    if keep != img.bounds() {
                        img = img.cropped(keep);
                    }
                }
                img
            }
        }
    }
}

/// The finished live stroke of the last release (FB-39): `bitmap.retouch` sent by the tool on
/// release takes it instead of computing the same stroke again, when it is for the same unchanged
/// bitmap element, the same points, parameters and pixel selection.
struct LiveResult {
    elem: Arc<fp_model::Element>,
    pts: Vec<Point>,
    prm: Retouch,
    sel: Option<Arc<Mask>>,
    image: RgbaImage,
}

thread_local! {
    static LIVE_RESULT: std::cell::RefCell<Option<LiveResult>> = const { std::cell::RefCell::new(None) };
}

impl RetouchStroke {
    /// Keeps the finished stroke (points `pts`, all fed) on the bitmap element `elem` for the
    /// `bitmap.retouch` that follows the release ([`take_live_result`]). Not with "Entire
    /// document" (the command renders the page itself).
    pub(crate) fn remember(&self, elem: Arc<fp_model::Element>, pts: &[Point]) {
        let entry = self.page.is_none().then(|| LiveResult {
            elem,
            pts: pts.to_vec(),
            prm: self.prm.clone(),
            sel: self.sel.clone(),
            image: self.finish(),
        });
        LIVE_RESULT.with(|c| *c.borrow_mut() = entry);
    }
}

/// The remembered live stroke if it is exactly this one (same element, unchanged since the press,
/// same points, parameters and selection); taken in any case, so it is used at most once.
pub(super) fn take_live_result(
    elem: &Arc<fp_model::Element>,
    pts: &[Point],
    prm: &Retouch,
    sel: Option<&Mask>,
) -> Option<RgbaImage> {
    let e = LIVE_RESULT.with(|c| c.borrow_mut().take())?;
    (Arc::ptr_eq(&e.elem, elem) && e.pts == pts && e.prm == *prm && e.sel.as_deref() == sel)
        .then_some(e.image)
}

fn copy_px(img: &mut RgbaImage, g: &Grow<[u8; 4]>, within: IRect) {
    let r = g.r.intersect(&within);
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            if let Some(p) = g.get(x, y) {
                img.set_pixel(x, y, Rgba8::new(p[0], p[1], p[2], p[3]));
            }
        }
    }
}

#[inline]
fn sel_w(sel: Option<&Mask>, x: i32, y: i32) -> f32 {
    sel.map(|m| m.get(x, y) as f32 / 255.0).unwrap_or(1.0)
}

/// Abwedler und Nachbelichter wie im Original: Die Belichtung richtet sich nach der Deckung des
/// ganzen Zugs (Abdruck [`Dab::stroke`], dicht gesetzt) und wirkt einmal auf die Ausgangspixel;
/// ein zweiter Zug verstärkt.
fn push_cover(
    c: &mut Cover,
    base: &RgbaImage,
    dab: &Dab,
    prm: &Retouch,
    sel: Option<&Mask>,
    stamps: &[Point],
) -> IRect {
    let orig = base.bounds();
    let dirty = stamps
        .iter()
        .fold(IRect::default(), |r, s| r.union(&dab.bounds(*s)))
        .intersect(&orig);
    if dirty.is_empty() {
        return dirty;
    }
    let next = grown(c.keep.r, dirty, orig, orig);
    c.keep.ensure(next, |_, _| 1.0);
    c.out.ensure(next, |x, y| base.get(x, y).unwrap_or([0; 4]));
    for s in stamps {
        dab.each(*s, |x, y, w| {
            if let Some(i) = c.keep.idx(x, y) {
                c.keep.v[i] *= 1.0 - w;
            }
        });
    }
    let dodge = prm.mode == "dodge";
    for y in dirty.top..dirty.bottom {
        for x in dirty.left..dirty.right {
            let (Some(i), Some(px)) = (c.keep.idx(x, y), base.get(x, y)) else {
                continue;
            };
            let mut cv = 1.0 - c.keep.v[i];
            if sel.is_some() {
                cv *= sel_w(sel, x, y);
            }
            c.out.v[i] = if cv <= 0.0 {
                px
            } else {
                // the coverage weakens the exposure (not the result)
                let f = |k: usize| {
                    tone_curve(dodge, prm.range, prm.strength * cv, px[k] as f32)
                        .round()
                        .clamp(0.0, 255.0) as u8
                };
                [f(0), f(1), f(2), px[3]]
            };
        }
    }
    dirty
}

/// Weichzeichner und Scharfzeichner wie im Original (PAR-X 04/02–04/14): Ein Pixel wird je Zug
/// ein- oder zweimal bearbeitet – einmal, wenn ein Mauspunkt höchstens `D/2 + 1,5` entfernt liegt,
/// zweimal, wenn es zwei sind (Original bei Intensität 50 an einer Kante 0|255: 15 bzw. 29 Stufen,
/// bei 100: 31 bzw. 52, bei 10: 3 bzw. 6; Einzelklick 15). Quer zum Zug schwächt die Deckung des
/// Abdrucks ([`Dab::stroke`], eckig/Rand 0 hart) die Wirkung ab. Weiche Verläufe bleiben (das
/// 3×3-Mittel weicht dort weniger als eine halbe Stufe ab).
fn push_passes(
    p: &mut Passes,
    base: &RgbaImage,
    dab: &Dab,
    prm: &Retouch,
    sel: Option<&Mask>,
    pts: &[Point],
    stamps: &[Point],
) -> IRect {
    let orig = base.bounds();
    let ext = p.ext;
    if ext.is_empty() {
        return IRect::default();
    }
    let blur = prm.mode == "blur";
    let radius = prm.size.max(1.0) / 2.0 + 1.5;
    // new dabs and mouse points
    let mut dirty = IRect::default();
    for s in stamps {
        let b = dab.bounds(*s);
        p.reach = p.reach.union(&b);
        dirty = dirty.union(&b);
    }
    let mut new_events = Vec::new();
    for q in pts {
        if p.last_event != Some(*q) {
            p.last_event = Some(*q);
            p.events += 1;
            new_events.push(*q);
        }
    }
    // the second mouse point changes the passes everywhere (a single click works once)
    let full = p.events > 1 && p.events - new_events.len() <= 1;
    let rr = radius.ceil() as i32 + 1;
    for q in &new_events {
        let (cx, cy) = (q.x.floor() as i32, q.y.floor() as i32);
        dirty = dirty.union(&IRect::new(cx - rr, cy - rr, cx + rr + 1, cy + rr + 1));
    }
    // buffers over the stroke area with a margin for the 3 x 3 reads and the hit discs
    let need = p.reach.union(&dirty).inflate(4, 4);
    let next = grown(p.keep.r, need, ext, ext);
    let init_px = |x: i32, y: i32| base.get(x, y).unwrap_or(OUTSIDE_PX);
    p.keep.ensure(next, |_, _| 1.0);
    p.hits.ensure(next, |_, _| 0);
    for g in &mut p.pass {
        g.ensure(next, init_px);
    }
    for s in stamps {
        dab.each(*s, |x, y, w| {
            if let Some(i) = p.keep.idx(x, y) {
                p.keep.v[i] *= 1.0 - w;
            }
        });
    }
    for q in &new_events {
        let (cx, cy) = (q.x.floor() as i32, q.y.floor() as i32);
        let r = IRect::new(cx - rr, cy - rr, cx + rr + 1, cy + rr + 1).intersect(&p.hits.r);
        for y in r.top..r.bottom {
            for x in r.left..r.right {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                if (q.x - fx).hypot(q.y - fy) <= radius
                    && let Some(i) = p.hits.idx(x, y)
                {
                    p.hits.v[i] = (p.hits.v[i] + 1).min(3);
                }
            }
        }
    }
    let work = p.reach.intersect(&ext);
    let region = if full { work } else { dirty.intersect(&work) };
    if region.is_empty() {
        return region;
    }
    let gain = if blur { BLUR_GAIN } else { SHARPEN_DAB_GAIN } * prm.strength;
    let multi = p.events > 1;
    // passes: a single click once; a fast stroke hits a spot one or two times in the original,
    // depending on where the mouse points lie - here one and a half times; a slow stroke (≥ 3
    // mouse points in reach) three times (FV-11)
    let weight = |h: u8, round: usize| -> f32 {
        match round {
            1 => 1.0,
            2 if h >= 3 => 1.0,
            2 if multi => 0.5,
            3 if h >= 3 => 1.0,
            _ => 0.0,
        }
    };
    for round in 1..=3usize {
        let area = region
            .inflate(round as i32 - 1, round as i32 - 1)
            .intersect(&work);
        let (done, rest) = p.pass.split_at_mut(round - 1);
        let out = &mut rest[0];
        // pass k reads pass k - 1; pass 1 the bitmap with the transparent white ring
        let src: Option<&Grow<[u8; 4]>> = done.last();
        let read = |x: i32, y: i32| -> [f32; 4] {
            match src.and_then(|g| g.get(x, y)) {
                Some(v) => v.map(|c| c as f32),
                None => base
                    .get(x, y)
                    .map(|v| v.map(|c| c as f32))
                    .unwrap_or(OUTSIDE),
            }
        };
        for y in area.top..area.bottom {
            for x in area.left..area.right {
                let Some(i) = p.keep.idx(x, y) else { continue };
                let prev = read(x, y).map(|v| v as u8);
                let rw = weight(p.hits.v[i].max(1), round);
                let mut c = (1.0 - p.keep.v[i]) * rw;
                if rw > 0.0 && sel.is_some() {
                    c *= sel_w(sel, x, y);
                }
                if rw <= 0.0 || c <= 0.0 {
                    out.v[i] = prev;
                    continue;
                }
                let px = read(x, y);
                let mut acc = [0f32; 4];
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let q = read(x + dx, y + dy);
                        for k in 0..4 {
                            acc[k] += q[k];
                        }
                    }
                }
                let a = acc.map(|v| v / 9.0);
                let t = c * gain;
                let o = if blur {
                    [0, 1, 2, 3].map(|k| px[k] + (a[k] - px[k]) * t)
                } else {
                    [
                        px[0] + (px[0] - a[0]) * t,
                        px[1] + (px[1] - a[1]) * t,
                        px[2] + (px[2] - a[2]) * t,
                        px[3],
                    ]
                };
                let mut v = o.map(|v| v.round().clamp(0.0, 255.0) as u8);
                if blur && !in_rect(&orig, x, y) {
                    // new edge pixels: coverage rounded down, dropped under 2/255 (FV-11)
                    v[3] = if o[3] < 2.0 {
                        0
                    } else {
                        o[3].floor().clamp(0.0, 255.0) as u8
                    };
                }
                out.v[i] = v;
            }
        }
    }
    region.inflate(2, 2).intersect(&work)
}

/// Verwischen wie im Original (PAR-X 04/09–04/11): Abdrücke je Pixel, jeder zieht den Inhalt um
/// den Weg seit dem vorigen Abdruck mit (Druck 50 → 47 %), das Zwischenergebnis wird je Abdruck auf
/// ganze Stufen gerundet. Harte Kanten wandern so ≈ 14 px in Zugrichtung, weiche Verläufe bleiben
/// (die Änderung je Abdruck liegt unter einer halben Stufe). „Farbe verwischen“ mischt die Farbe
/// am Zuganfang ein und lässt sie mit dem Zug auslaufen; „Gesamtes Dokument“ nimmt die sichtbare
/// Seite (`page`, mit Leinwand) statt nur der Bitmap auf.
///
/// Past the bitmap edge (FB-15, RE spec 57 section 3; FB-40): when the stroke may grow the
/// bitmap (`Smear::grow`: not "Confine to boundaries"; with "Entire document" only a stroke that
/// starts on the bitmap) the work area is the whole reach of the stroke; pixels outside the bitmap
/// start transparent (or as the page with "Entire document"), so the smear carries the picked-up
/// colour over the edge with the decaying weight of the drag and fades out semi-transparent; the
/// bitmap then grows to the pixels that got coverage (with "Entire document" to the whole reach,
/// filled with the page). A stroke from outside drags transparency in, which grows nothing.
/// Otherwise only the bitmap is worked on; "Confine to boundaries" keeps the outside out entirely
/// (the clipped behaviour). Colours mix premultiplied by their alpha, so transparent pixels carry
/// no colour of their own (identical to straight mixing for opaque pixels).
fn push_smear(
    s: &mut Smear,
    base: &RgbaImage,
    dab: &Dab,
    prm: &Retouch,
    sel: Option<&Mask>,
    page: Option<&RgbaImage>,
    stamps: &[Point],
) -> IRect {
    let orig = base.bounds();
    let grow = s.grow == Some(true);
    let mut dirty = IRect::default();
    for st in stamps {
        let b = dab.bounds(*st);
        s.reach = s.reach.union(&b);
        dirty = dirty.union(&b);
    }
    let limit = if grow { SANE } else { orig };
    let work = s.reach.inflate(2, 2).intersect(&limit);
    // the start value of a pixel: the page ("Entire document") or the bitmap, outside transparent
    let init = |x: i32, y: i32| -> [f32; 4] {
        match page.and_then(|pg| pg.get(x, y)) {
            Some(q) => q,
            None => base.get(x, y).unwrap_or([0; 4]),
        }
        .map(|c| c as f32)
    };
    let pad = prm.size.ceil() as i32 + 32;
    let first = if grow { orig.inflate(pad, pad) } else { orig };
    let next = grown(s.buf.r, work, limit, first);
    s.buf.ensure(next, init);
    s.touched.ensure(next, |_, _| false);
    // source outside the buffers: page or bitmap; outside the bitmap transparent (drags
    // transparency in), with "Confine to boundaries" nothing
    let confine = prm.confine;
    let outside = |x: i32, y: i32| -> Option<[f32; 4]> {
        page.and_then(|pg| pg.get(x, y))
            .or_else(|| base.get(x, y))
            .map(|p| p.map(|v| v as f32))
            .or(if confine { None } else { Some([0.0; 4]) })
    };
    let pressure = 0.94 * prm.strength;
    for st in stamps {
        let st = *st;
        let mut changes: Vec<(usize, [f32; 4])> = Vec::new();
        {
            let buf = &s.buf;
            let sample = |x: i32, y: i32| -> Option<[f32; 4]> {
                if in_rect(&work, x, y) {
                    buf.get(x, y)
                } else {
                    outside(x, y)
                }
            };
            let prev = s.prev;
            let color_w = s.color_w;
            dab.each(st, |x, y, w0| {
                if !in_rect(&work, x, y) {
                    return;
                }
                let Some(i) = buf.idx(x, y) else { return };
                let mut w = w0 * pressure;
                if sel.is_some() {
                    w *= sel_w(sel, x, y);
                }
                if w <= 0.0 {
                    return;
                }
                let px = buf.v[i];
                let ppx = premul(px);
                let mut out = ppx;
                if let Some(pv) = prev {
                    // content at the place of the previous dab (bilinear)
                    let (fx, fy) = (x as f32 - (st.x - pv.x), y as f32 - (st.y - pv.y));
                    let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
                    let (ax, ay) = (fx - x0 as f32, fy - y0 as f32);
                    let g = |xx: i32, yy: i32| premul(sample(xx, yy).unwrap_or(px));
                    let (q00, q10, q01, q11) =
                        (g(x0, y0), g(x0 + 1, y0), g(x0, y0 + 1), g(x0 + 1, y0 + 1));
                    let q = [0, 1, 2, 3].map(|k| {
                        q00[k] * (1.0 - ax) * (1.0 - ay)
                            + q10[k] * ax * (1.0 - ay)
                            + q01[k] * (1.0 - ax) * ay
                            + q11[k] * ax * ay
                    });
                    out = [0, 1, 2, 3].map(|k| ppx[k] + (q[k] - ppx[k]) * w);
                }
                if let Some(c) = prm.finger {
                    let t = w * color_w;
                    let cv = [c.r as f32, c.g as f32, c.b as f32, 255.0];
                    out = [0, 1, 2, 3].map(|k| out[k] + (cv[k] - out[k]) * t);
                }
                let v = unpremul(out).map(|v| v.round().clamp(0.0, 255.0));
                // a transparent pixel that stays transparent keeps its stored colour
                let v = if v[3] == 0.0 && px[3] == 0.0 { px } else { v };
                changes.push((i, v));
            });
        }
        for (i, v) in changes {
            s.buf.v[i] = v;
            s.touched.v[i] = true;
        }
        if s.prev.is_some() {
            s.color_w *= 0.93;
        }
        s.prev = Some(st);
    }
    dirty.intersect(&work)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u64);
    impl Lcg {
        fn f(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) as f32) / (1u64 << 31) as f32
        }
    }

    fn prm(mode: &str, k: usize) -> Retouch {
        Retouch {
            mode: mode.to_string(),
            size: [1.0, 3.0, 5.0, 13.0, 16.0, 22.0, 40.0][k % 7],
            round: k % 5 != 3,
            soft: [0.0, 0.02, 0.5, 1.0][k % 4],
            strength: [0.1, 0.5, 1.0][k % 3],
            range: (k % 3) as i64,
            spacing: 20.0,
            offset: (0.0, 0.0),
            from: Rgba8::new(0, 0, 0, 255),
            to: Rgba8::new(0, 0, 0, 255),
            colorize: false,
            tolerance: 50.0,
            finger: (k % 4 == 1).then(|| Rgba8::new(10, 200, 90, 255)),
            confine: k % 6 == 2,
            breaks: Vec::new(),
        }
    }

    /// FB-39: the stroke fed point by point (or in chunks) gives exactly the bitmap of the stroke
    /// fed at once (what `bitmap.retouch` stores), and the preview pixels after the last push are
    /// that bitmap; also across the bitmap edge, with a pixel selection, "Smudge color", "Entire
    /// document" and "Confine to boundaries".
    #[test]
    fn incremental_pushes_equal_the_stroke_at_once() {
        let mut rng = Lcg(11);
        let r = IRect::new(10, 20, 90, 80);
        let mut base = RgbaImage::new(r).expect("img");
        for y in r.top..r.bottom {
            for x in r.left..r.right {
                let a = if (x / 7 + y / 5) % 5 == 0 {
                    0
                } else if x % 11 == 0 {
                    120
                } else {
                    255
                };
                let c = Rgba8::new(
                    (x * 3 % 256) as u8,
                    (y * 7 % 256) as u8,
                    ((x ^ y) % 256) as u8,
                    a,
                );
                base.set_pixel(x, y, c);
            }
        }
        let base = Arc::new(base);
        let mut page = RgbaImage::new(IRect::new(0, 0, 120, 110)).expect("page");
        for y in 0..110 {
            for x in 0..120 {
                page.set_pixel(x, y, Rgba8::new(200, (x % 256) as u8, 30, 255));
            }
        }
        let page = Arc::new(page);
        let mut sel = Mask::new(IRect::new(20, 25, 70, 75)).expect("mask");
        for y in 25..75 {
            for x in 20..70 {
                sel.set(x, y, ((x * 9 + y * 5) % 256) as u8);
            }
        }
        let sel = Arc::new(sel);
        let mut cases = 0;
        for mode in ["blur", "sharpen", "dodge", "burn", "smudge"] {
            for k in 0..60 {
                let n = 1 + (rng.f() * 30.0) as usize;
                let mut pts = Vec::new();
                let (mut x, mut y) = (rng.f() * 100.0, rng.f() * 100.0);
                for _ in 0..n {
                    pts.push(Point::new(x, y));
                    if rng.f() < 0.15 {
                        continue; // repeated point
                    }
                    x += (rng.f() - 0.5) * 14.0;
                    y += (rng.f() - 0.5) * 14.0;
                }
                let sel = (k % 3 == 1).then(|| Arc::clone(&sel));
                let page = (mode == "smudge" && k % 5 == 4).then(|| Arc::clone(&page));
                // FB-31: every other case is a resumed stroke with one to three breaks
                let mut breaks: Vec<usize> = Vec::new();
                if k % 2 == 1 && n > 2 {
                    for _ in 0..1 + (rng.f() * 3.0) as usize {
                        breaks.push(1 + (rng.f() * (n - 1) as f32) as usize);
                    }
                    breaks.sort_unstable();
                    breaks.dedup();
                }
                let mk = || {
                    RetouchStroke::new(Arc::clone(&base), prm(mode, k), sel.clone(), page.clone())
                        .expect("mode")
                };
                let mut once = mk();
                once.feed(&pts, &breaks);
                let want = once.finish();
                let mut inc = mk();
                let mut i = 0;
                while i < pts.len() {
                    let j = (i + 1 + (rng.f() * 3.0) as usize).min(pts.len());
                    // the live tool feeds the points so far with the breaks so far
                    let known: Vec<usize> = breaks.iter().copied().filter(|b| *b < j).collect();
                    inc.feed(&pts[..j], &known);
                    i = j;
                }
                assert!(inc.finish() == want, "{mode} case {k}: result");
                let u = want.bounds().union(&base.bounds());
                let norm = |p: [u8; 4]| if p[3] == 0 { [0; 4] } else { p };
                for yy in u.top..u.bottom {
                    for xx in u.left..u.right {
                        assert_eq!(
                            norm(inc.pixel(xx, yy)),
                            norm(want.get(xx, yy).unwrap_or([0; 4])),
                            "{mode} case {k}: preview pixel {xx}, {yy}"
                        );
                    }
                }
                cases += 1;
            }
        }
        assert_eq!(cases, 300);
    }

    /// FB-31: a resumed stroke. Smudge starts every part afresh: with "Confine to boundaries" the
    /// stroke with a break equals the first part followed by a new stroke on its result. Dodge
    /// does not join the parts: the gap between the end of one part and the press of the next
    /// stays untouched, and the overlap of two parts is exposed once (the whole stroke once).
    #[test]
    fn breaks_start_new_parts() {
        let r = IRect::new(0, 0, 120, 40);
        let mut base = RgbaImage::new(r).expect("img");
        for y in r.top..r.bottom {
            for x in r.left..r.right {
                let v = if (x / 6) % 2 == 0 { 40 } else { 200 };
                base.set_pixel(x, y, Rgba8::new(v, 120, 255 - v, 255));
            }
        }
        let base = Arc::new(base);
        let pts = [
            Point::new(10.0, 20.0),
            Point::new(30.0, 20.0),
            Point::new(45.0, 22.0),
            Point::new(75.0, 18.0),
            Point::new(100.0, 20.0),
        ];
        // smudge, confined: one stroke with a break at 3 = part 1, then a fresh stroke
        let mut smudge = prm("smudge", 0);
        smudge.size = 9.0;
        smudge.confine = true;
        smudge.finger = None;
        let mut whole =
            RetouchStroke::new(Arc::clone(&base), smudge.clone(), None, None).expect("smudge");
        whole.feed(&pts, &[3]);
        let mut first =
            RetouchStroke::new(Arc::clone(&base), smudge.clone(), None, None).expect("smudge");
        first.push(&pts[..3]);
        let mut second = RetouchStroke::new(Arc::new(first.finish()), smudge.clone(), None, None)
            .expect("smudge");
        second.push(&pts[3..]);
        assert!(
            whole.finish() == second.finish(),
            "smudge parts start afresh"
        );
        let mut joined = RetouchStroke::new(Arc::clone(&base), smudge, None, None).expect("smudge");
        joined.push(&pts);
        assert!(
            joined.finish() != whole.finish(),
            "the break changes the smudge"
        );
        // dodge: the gap between (45, 22) and (75, 18) is only reached when the parts are joined
        let mut dodge = prm("dodge", 0);
        dodge.size = 5.0;
        dodge.strength = 1.0;
        let mut parts =
            RetouchStroke::new(Arc::clone(&base), dodge.clone(), None, None).expect("dodge");
        parts.feed(&pts, &[3]);
        let mut joined =
            RetouchStroke::new(Arc::clone(&base), dodge.clone(), None, None).expect("dodge");
        joined.push(&pts);
        let (gx, gy) = (60, 20);
        assert_eq!(
            parts.pixel(gx, gy),
            base.get(gx, gy).expect("px"),
            "gap untouched"
        );
        assert_ne!(
            joined.pixel(gx, gy),
            base.get(gx, gy).expect("px"),
            "joined stroke reaches the gap"
        );
        // the dab positions are those of the parts on their own
        let mut w = DabWalker::new(1.0);
        let mut got = Vec::new();
        for (i, p) in pts.iter().enumerate() {
            if i == 3 {
                w = DabWalker::new(1.0);
            }
            w.push(*p, &mut got);
        }
        let want: Vec<Point> = crate::paint::split_segments(&pts, &[3])
            .into_iter()
            .flat_map(|seg| super::super::bitmap::dab_points(seg, 1.0))
            .collect();
        assert_eq!(got, want);
        // overlap once: a part retracing the first part exposes it like the first part alone
        let back = [
            Point::new(10.0, 20.0),
            Point::new(30.0, 20.0),
            Point::new(10.0, 20.0),
            Point::new(30.0, 20.0),
        ];
        let mut once =
            RetouchStroke::new(Arc::clone(&base), dodge.clone(), None, None).expect("dodge");
        once.push(&back[..2]);
        let mut twice = RetouchStroke::new(Arc::clone(&base), dodge, None, None).expect("dodge");
        twice.feed(&back, &[2]);
        let soft = once.pixel(20, 22);
        assert!(
            twice.pixel(20, 22)[0] >= soft[0],
            "the coverage of both parts counts once"
        );
        assert_eq!(
            twice.pixel(20, 20),
            once.pixel(20, 20),
            "full coverage stays full"
        );
    }

    /// The walker places the dabs exactly like `dab_points` on the whole sequence.
    #[test]
    fn dab_walker_matches_dab_points() {
        let pts = [
            Point::new(1.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(4.3, 2.0),
            Point::new(4.3, 9.7),
            Point::new(0.2, 0.1),
        ];
        for step in [0.1, 1.0, 2.6] {
            let want = super::super::bitmap::dab_points(&pts, step);
            let mut w = DabWalker::new(step);
            let mut got = Vec::new();
            for p in pts {
                w.push(p, &mut got);
            }
            assert_eq!(got, want, "step {step}");
        }
    }
}
