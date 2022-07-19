use std::{num::NonZeroU32, rc::Rc};

use super::tree;
use crate::{
    crate_util::FxHashSet,
    render::{FrameBinding, FrameValue},
    units::*,
    WidgetId,
};

// The QuadTree is implemented as a grid of 2048px squares that is each an actual quad-tree.
//
// The actual quad-trees can have any number of "leaves" in each level, depending on the `QLevel`.

const MIN_QUAD: Px = Px(128);
const ROOT_QUAD: Px = Px(2048);

/// Items can be inserted in multiple quads, but only of the same level or larger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct QLevel(u8);
impl QLevel {
    fn from_size(size: PxSize) -> Self {
        Self::from_length(size.width.min(size.height))
    }

    fn from_length(length: Px) -> Self {
        if length <= MIN_QUAD {
            QLevel(250)
        } else if length <= Px(256) {
            QLevel(251)
        } else if length <= Px(512) {
            QLevel(253)
        } else if length <= Px(1024) {
            QLevel(254)
        } else {
            QLevel(255)
        }
    }
}

#[derive(Clone, Copy)]
struct PxSquare {
    origin: PxPoint,
    length: Px,
}

type PxBox = euclid::Box2D<Px, ()>;

pub(super) struct QuadTree {
    grid: Vec<QuadRoot>,
    bounds: PxBox,
}
impl QuadTree {
    pub(super) fn new() -> Self {
        Self {
            grid: vec![QuadRoot::new(PxPoint::zero())],
            bounds: PxBox::zero(),
        }
    }

    pub(super) fn bounds(&self) -> PxBox {
        self.bounds
    }

    pub(super) fn insert(&mut self, item: tree::NodeId, item_bounds: PxRect) {
        let item_level = QLevel::from_size(item_bounds.size);
        let item_bounds = item_bounds.to_box2d();

        if self.is_empty() {
            self.bounds = item_bounds;
        } else {
            self.bounds.min = self.bounds.min.min(item_bounds.min);
            self.bounds.max = self.bounds.max.max(item_bounds.max);
        }

        for r in &mut self.grid {
            if r.root_bounds().contains_box(item_bounds) {
                r.insert(item, item_bounds, item_level);
                return;
            }
        }

        let q = ROOT_QUAD.0;
        let min = (item_bounds.min.x.0 / q, item_bounds.min.y.0 / q);
        let max = (item_bounds.max.x.0 / q, item_bounds.max.y.0 / q);
        for cell_x in min.0..=max.0 {
            'next_cell: for cell_y in min.1..=max.1 {
                let root_origin = PxPoint::new(Px(cell_x * q), Px(cell_y * q));

                for r in &mut self.grid {
                    if r.grid_origin == root_origin {
                        r.insert(item, item_bounds, item_level);
                        continue 'next_cell;
                    }
                }

                let mut root = QuadRoot::new(root_origin);
                root.insert(item, item_bounds, item_level);
                self.grid.push(root);
            }
        }
    }

    pub(super) fn quad_query<'a>(self: Rc<Self>, include: impl FnMut(PxBox) -> bool + 'a) -> impl Iterator<Item = tree::NodeId> + 'a {
        QuadQueryIter::new(include, self)
    }

    pub(super) fn quad_query_dedup<'a>(self: Rc<Self>, include: impl FnMut(PxBox) -> bool + 'a) -> impl Iterator<Item = tree::NodeId> + 'a {
        let mut visited = FxHashSet::default();
        self.quad_query(include).filter(move |n| visited.insert(*n))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }
}

struct QuadStorage {
    nodes: Vec<QuadNodeData>,
    items: Vec<QuadItemData>,
}

struct QuadRoot {
    grid_origin: PxPoint,
    storage: QuadStorage,
}

#[derive(Clone, Copy)]
struct QuadNode(u32);

impl QuadRoot {
    pub(super) fn new(grid_origin: PxPoint) -> Self {
        Self {
            storage: QuadStorage::new(),
            grid_origin,
        }
    }

    fn root_bounds(&self) -> PxSquare {
        PxSquare {
            origin: self.grid_origin,
            length: ROOT_QUAD,
        }
    }

    fn insert(&mut self, item: tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        let root_bounds = self.root_bounds();
        QuadNode(0).insert(&mut self.storage, root_bounds, item, item_bounds, item_level);
    }
}

impl QuadNode {
    fn insert(self, storage: &mut QuadStorage, self_bounds: PxSquare, item: tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        if let Some(q) = self_bounds.split() {
            let q_level = QLevel::from_length(q[0].length);
            if q_level >= item_level {
                for (quad, q_bounds) in storage.split(self.0).zip(q) {
                    if q_bounds.intersects(item_bounds) {
                        QuadNode(quad).insert(storage, q_bounds, item, item_bounds, item_level);
                    }
                }
                return;
            }
        }
        storage.push_item(self.0, item);
    }
}

impl PxSquare {
    fn split(mut self) -> Option<[Self; 4]> {
        self.length /= Px(2);

        if self.length >= MIN_QUAD {
            let mut r = [self; 4];
            r[1].origin.x += self.length;
            r[2].origin.y += self.length;
            r[3].origin += PxVector::splat(self.length);
            Some(r)
        } else {
            None
        }
    }

    fn to_box(self) -> PxBox {
        let max = self.origin + PxVector::splat(self.length);
        PxBox::new(self.origin, max)
    }

    fn intersects(self, b: PxBox) -> bool {
        self.to_box().intersects(&b)
    }

    fn contains_box(self, b: PxBox) -> bool {
        self.to_box().contains_box(&b)
    }
}

impl QuadStorage {
    fn new() -> Self {
        let mut r = QuadStorage {
            nodes: vec![],
            items: vec![],
        };
        r.nodes.push(QuadNodeData {
            children: None,
            items: None,
        });
        r
    }

    fn split(&mut self, node: u32) -> impl Iterator<Item = u32> {
        if let Some(c) = self.nodes[node as usize].children {
            let c = c.get() - 1;
            c..c + 4
        } else {
            let c = self.nodes.len() as u32;
            for _ in 0..4 {
                self.nodes.push(QuadNodeData::default());
            }
            self.nodes[node as usize].children = NonZeroU32::new(c + 1);
            c..c + 4
        }
    }

    fn children(&self, node: u32) -> std::ops::Range<u32> {
        if let Some(c) = self.nodes[node as usize].children {
            let c = c.get() - 1;
            c..c + 4
        } else {
            0..0
        }
    }

    fn push_item(&mut self, node: u32, item: tree::NodeId) {
        let item_i = NonZeroU32::new(self.items.len() as u32 + 1);
        self.items.push(QuadItemData { item, next: None });

        if let Some(ii) = self.nodes[node as usize].items {
            let mut i = ii.get() - 1;
            while let Some(ii) = self.items[i as usize].next {
                i = ii.get() - 1;
            }
            self.items[i as usize].next = item_i;
        } else {
            self.nodes[node as usize].items = item_i;
        }
    }
}
#[derive(Default)]
struct QuadNodeData {
    children: Option<NonZeroU32>,
    items: Option<NonZeroU32>,
}
struct QuadItemData {
    item: tree::NodeId,
    next: Option<NonZeroU32>,
}

type NodeZipQuadIter = std::iter::Zip<std::ops::Range<u32>, std::array::IntoIter<PxSquare, 4>>;

struct QuadQueryIter<Q> {
    query: Q,

    tree: Rc<QuadTree>,
    cell: usize,
    node_stack: Vec<NodeZipQuadIter>,
    node: NodeZipQuadIter,
    item: Option<NonZeroU32>,
}
impl<Q: FnMut(PxBox) -> bool> QuadQueryIter<Q> {
    fn new(query: Q, tree: Rc<QuadTree>) -> Self {
        Self {
            query,
            tree,
            cell: usize::MAX,
            node_stack: Vec::with_capacity(8),
            node: (0u32..0u32).zip(
                [PxSquare {
                    origin: PxPoint::zero(),
                    length: Px(0),
                }; 4],
            ),
            item: None,
        }
    }
}
impl<Q: FnMut(PxBox) -> bool> Iterator for QuadQueryIter<Q> {
    type Item = tree::NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(i) = self.item {
                // next item.
                let item = &self.tree.grid[self.cell].storage.items[(i.get() - 1) as usize];
                self.item = item.next;
                return Some(item.item);
            } else if let Some((q, q_bounds)) = self.node.next() {
                // next quad:
                if (self.query)(q_bounds.to_box()) {
                    let node = &self.tree.grid[self.cell].storage.nodes[q as usize];
                    if node.items.is_some() {
                        // next quad items.
                        self.item = node.items;
                    }
                    if node.children.is_some() {
                        // next inner quad.
                        self.node_stack.push(self.node.clone());
                        self.node = self.tree.grid[self.cell].storage.children(q).zip(q_bounds.split().unwrap());
                    }
                }

                continue;
            } else if let Some(q) = self.node_stack.pop() {
                // return to parent nodes.
                self.node = q;

                continue;
            } else if self.cell < self.tree.grid.len() - 1 || self.cell == usize::MAX {
                // next quad-tree.

                self.cell = self.cell.wrapping_add(1);

                let r = &self.tree.grid[self.cell];
                if (self.query)(r.root_bounds().to_box()) {
                    self.node = r.storage.children(0).zip(r.root_bounds().split().unwrap());
                }

                continue;
            } else {
                return None;
            }
        }
    }
}

/// Represents hit-test regions of a widget inner.
#[derive(Debug, Default)]
pub(crate) struct HitTestClips {
    items: Vec<HitTestItem>,
}
impl HitTestClips {
    /// Returns `true` if any hit-test clip is registered for this widget.
    pub fn is_hit_testable(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn push_rect(&mut self, rect: euclid::Box2D<Px, ()>) {
        self.items.push(HitTestItem::Hit(HitTestPrimitive::Rect(rect)));
    }

    pub fn push_clip_rect(&mut self, clip_rect: euclid::Box2D<Px, ()>, clip_out: bool) {
        self.items.push(HitTestItem::Clip(HitTestPrimitive::Rect(clip_rect), clip_out));
    }

    pub fn push_rounded_rect(&mut self, rect: euclid::Box2D<Px, ()>, radii: PxCornerRadius) {
        if radii == PxCornerRadius::zero() {
            self.push_rect(rect);
        } else {
            self.items.push(HitTestItem::Hit(HitTestPrimitive::RoundedRect(rect, radii)));
        }
    }

    pub fn push_clip_rounded_rect(&mut self, clip_rect: euclid::Box2D<Px, ()>, radii: PxCornerRadius, clip_out: bool) {
        if radii == PxCornerRadius::zero() {
            self.push_clip_rect(clip_rect, clip_out);
        } else {
            self.items
                .push(HitTestItem::Clip(HitTestPrimitive::RoundedRect(clip_rect, radii), clip_out));
        }
    }

    pub fn push_ellipse(&mut self, center: PxPoint, radii: PxSize) {
        self.items.push(HitTestItem::Hit(HitTestPrimitive::Ellipse(center, radii)));
    }

    pub fn push_clip_ellipse(&mut self, center: PxPoint, radii: PxSize, clip_out: bool) {
        self.items
            .push(HitTestItem::Clip(HitTestPrimitive::Ellipse(center, radii), clip_out));
    }

    pub fn pop_clip(&mut self) {
        self.items.push(HitTestItem::PopClip);
    }

    pub fn push_transform(&mut self, transform: FrameBinding<RenderTransform>) {
        self.items.push(HitTestItem::Transform(transform))
    }

    pub fn pop_transform(&mut self) {
        self.items.push(HitTestItem::PopTransform);
    }

    pub fn push_child(&mut self, widget: WidgetId) {
        self.items.push(HitTestItem::Child(widget));
    }

    /// Hit-test the `point` against the items, returns the relative Z of the hit.
    pub fn hit_test_z(&self, inner_transform: &RenderTransform, window_point: PxPoint) -> RelativeHitZ {
        let mut z = RelativeHitZ::NoHit;
        let mut child = None;

        let mut transform_stack = vec![];
        let mut current_transform = inner_transform;
        let mut local_point = match inv_transform_point(current_transform, window_point) {
            Some(p) => p,
            None => return RelativeHitZ::NoHit,
        };

        let mut items = self.items.iter();

        'hit_test: while let Some(item) = items.next() {
            match item {
                HitTestItem::Hit(prim) => {
                    if prim.contains(local_point) {
                        z = if let Some(inner) = child {
                            RelativeHitZ::Over(inner)
                        } else {
                            RelativeHitZ::Back
                        };
                    }
                }

                HitTestItem::Clip(prim, clip_out) => {
                    let skip = match clip_out {
                        true => prim.contains(local_point),
                        false => !prim.contains(local_point),
                    };

                    if skip {
                        // clip excluded point, skip all clipped shapes.
                        let mut clip_depth = 0;
                        'skip_clipped: for item in items.by_ref() {
                            match item {
                                HitTestItem::Clip(_, _) => {
                                    clip_depth += 1;
                                }
                                HitTestItem::PopClip => {
                                    if clip_depth == 0 {
                                        continue 'hit_test;
                                    }
                                    clip_depth -= 1;
                                }
                                HitTestItem::Child(w) => {
                                    child = Some(*w);
                                    continue 'skip_clipped;
                                }
                                _ => continue 'skip_clipped,
                            }
                        }
                    }
                }
                HitTestItem::PopClip => continue 'hit_test,

                HitTestItem::Transform(t) => {
                    let t = match t {
                        FrameBinding::Value(t) | FrameBinding::Binding(_, t) => t,
                    };
                    match inv_transform_point(t, local_point) {
                        Some(p) => {
                            // transform is valid, push previous transform and replace the local point.
                            transform_stack.push((current_transform, local_point));
                            current_transform = t;
                            local_point = p;
                        }
                        None => {
                            // non-invertible transform, skip all transformed shapes.
                            let mut transform_depth = 0;
                            'skip_transformed: for item in items.by_ref() {
                                match item {
                                    HitTestItem::Transform(_) => {
                                        transform_depth += 1;
                                    }
                                    HitTestItem::PopTransform => {
                                        if transform_depth == 0 {
                                            continue 'hit_test;
                                        }
                                        transform_depth -= 1;
                                    }
                                    HitTestItem::Child(w) => {
                                        child = Some(*w);
                                        continue 'skip_transformed;
                                    }
                                    _ => continue 'skip_transformed,
                                }
                            }
                        }
                    }
                }
                HitTestItem::PopTransform => {
                    (current_transform, local_point) = transform_stack.pop().unwrap();
                }

                HitTestItem::Child(w) => {
                    child = Some(*w);
                }
            }
        }

        if let (RelativeHitZ::Over(w), Some(c)) = (z, child) {
            if w == c {
                return RelativeHitZ::Front;
            }
        }
        z
    }

    pub fn update_transform(&mut self, value: FrameValue<RenderTransform>) {
        for item in &mut self.items {
            if let HitTestItem::Transform(FrameBinding::Binding(key, t)) = item {
                if *key == value.key {
                    *t = value.value;
                    break;
                }
            }
        }
    }

    /// Returns `true` if a clip that affects the `child` clips out the `window_point`.
    pub fn clip_child(&self, child: WidgetId, inner_transform: &RenderTransform, window_point: PxPoint) -> bool {
        false
    }
}

/// Hit-test result on a widget relative to it's descendants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeHitZ {
    /// Widget was not hit.
    NoHit,
    /// Widget was hit on a hit-test shape rendered before the widget descendants.
    Back,
    /// Widget was hit on a hit-test shape rendered after the child.
    Over(WidgetId),
    /// Widget was hit on a hit-test shape rendered after the widget descendants.
    Front,
}

#[derive(Debug)]
enum HitTestPrimitive {
    Rect(euclid::Box2D<Px, ()>),
    RoundedRect(euclid::Box2D<Px, ()>, PxCornerRadius),
    Ellipse(PxPoint, PxSize),
}
impl HitTestPrimitive {
    fn contains(&self, point: PxPoint) -> bool {
        match self {
            HitTestPrimitive::Rect(r) => r.contains(point),
            HitTestPrimitive::RoundedRect(rect, radii) => rounded_rect_contains(rect, radii, point),
            HitTestPrimitive::Ellipse(center, radii) => ellipse_contains(*radii, *center, point),
        }
    }
}
#[derive(Debug)]
enum HitTestItem {
    Hit(HitTestPrimitive),

    Clip(HitTestPrimitive, bool),
    PopClip,

    Transform(FrameBinding<RenderTransform>),
    PopTransform,

    Child(WidgetId),
}

fn rounded_rect_contains(rect: &euclid::Box2D<Px, ()>, radii: &PxCornerRadius, point: PxPoint) -> bool {
    if !rect.contains(point) {
        return false;
    }

    let top_left_center = rect.min + radii.top_left.to_vector();
    if top_left_center.x > point.x && top_left_center.y > point.y && !ellipse_contains(radii.top_left, top_left_center, point) {
        return false;
    }

    let bottom_right_center = rect.max - radii.bottom_right.to_vector();
    if bottom_right_center.x < point.x
        && bottom_right_center.y < point.y
        && !ellipse_contains(radii.bottom_right, bottom_right_center, point)
    {
        return false;
    }

    let top_right = PxPoint::new(rect.max.x, rect.min.y);
    let top_right_center = top_right + PxVector::new(-radii.top_right.width, radii.top_right.height);
    if top_right_center.x < point.x && top_right_center.y > point.y && !ellipse_contains(radii.top_right, top_right_center, point) {
        return false;
    }

    let bottom_left = PxPoint::new(rect.min.x, rect.max.y);
    let bottom_left_center = bottom_left + PxVector::new(radii.bottom_left.width, -radii.bottom_left.height);
    if bottom_left_center.x > point.x && bottom_left_center.y < point.y && !ellipse_contains(radii.bottom_left, bottom_left_center, point) {
        return false;
    }

    true
}

fn ellipse_contains(radii: PxSize, center: PxPoint, point: PxPoint) -> bool {
    let h = center.x.0 as f64;
    let k = center.y.0 as f64;

    let a = radii.width.0 as f64;
    let b = radii.height.0 as f64;

    let x = point.x.0 as f64;
    let y = point.y.0 as f64;

    let p = ((x - h).powi(2) / a.powi(2)) + ((y - k).powi(2) / b.powi(2));

    p <= 1.0
}

fn inv_transform_point(t: &RenderTransform, point: PxPoint) -> Option<PxPoint> {
    t.inverse()?.transform_px_point(point)
}
