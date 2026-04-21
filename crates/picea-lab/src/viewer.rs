use std::{error::Error, fs, path::Path};

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use picea::{
    element::ID,
    tools::observability::{
        ContactSnapshot, DebugEdge, DebugLabel, DebugShape, LabArtifacts, LabPhase, LabPoint,
        ManifoldSnapshot, TraceEvent,
    },
};

use crate::recipes::{capture_recipe, BenchmarkScenario, RunRecipe};

pub struct ViewerModel {
    artifacts: LabArtifacts,
    recipe: RunRecipe,
    render_settings: RenderSettings,
    element_filter: Option<ID>,
    pair_filter: Option<[ID; 2]>,
    phase_filter: Option<LabPhase>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewerSummary {
    pub run_id: String,
    pub state_hash: String,
    pub element_count: usize,
    pub shape_count: usize,
    pub contact_count: usize,
    pub active_pair_count: usize,
    pub timeline_event_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderSettings {
    pub show_contacts: bool,
    pub show_normals: bool,
    pub show_manifold_labels: bool,
    pub show_overlay_text: bool,
    pub contact_radius: f32,
    pub normal_scale: f32,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            show_contacts: true,
            show_normals: true,
            show_manifold_labels: true,
            show_overlay_text: true,
            contact_radius: 4.0,
            normal_scale: 24.0,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

impl ViewerModel {
    pub fn from_artifacts(artifacts: LabArtifacts) -> Self {
        Self {
            recipe: infer_recipe_from_artifacts(&artifacts),
            render_settings: RenderSettings::default(),
            artifacts,
            element_filter: None,
            pair_filter: None,
            phase_filter: None,
        }
    }

    pub fn load_from_dir(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        LabArtifacts::read_from_dir(dir).map(Self::from_artifacts)
    }

    pub fn summary(&self) -> ViewerSummary {
        ViewerSummary {
            run_id: self.artifacts.final_snapshot.run_id.clone(),
            state_hash: self.artifacts.state_hash(),
            element_count: self.artifacts.final_snapshot.elements.len(),
            shape_count: self.artifacts.debug_render.shapes.len(),
            contact_count: self.artifacts.final_snapshot.contacts.len(),
            active_pair_count: self.artifacts.final_snapshot.active_pairs.len(),
            timeline_event_count: self.artifacts.trace_events.len(),
        }
    }

    pub fn artifacts(&self) -> &LabArtifacts {
        &self.artifacts
    }

    pub fn recipe(&self) -> &RunRecipe {
        &self.recipe
    }

    pub fn set_recipe(&mut self, recipe: RunRecipe) {
        self.recipe = recipe;
    }

    pub fn regenerate(&mut self) -> std::io::Result<()> {
        self.artifacts = capture_recipe(self.recipe.clone());
        self.clear_filters();
        Ok(())
    }

    pub fn render_settings(&self) -> &RenderSettings {
        &self.render_settings
    }

    pub fn render_settings_mut(&mut self) -> &mut RenderSettings {
        &mut self.render_settings
    }

    pub fn reset_view(&mut self) {
        self.render_settings.zoom = 1.0;
        self.render_settings.pan_x = 0.0;
        self.render_settings.pan_y = 0.0;
    }

    pub fn pan_view(&mut self, dx: f32, dy: f32) {
        self.render_settings.pan_x += dx;
        self.render_settings.pan_y += dy;
    }

    pub fn zoom_view(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.render_settings.zoom = (self.render_settings.zoom * factor).clamp(0.5, 3.0);
        }
    }

    pub fn phase_filter(&self) -> Option<LabPhase> {
        self.phase_filter
    }

    pub fn set_element_filter(&mut self, element_id: Option<ID>) {
        self.element_filter = element_id;
    }

    pub fn set_pair_filter(&mut self, pair: Option<[ID; 2]>) {
        self.pair_filter = pair;
    }

    pub fn set_phase_filter(&mut self, phase: Option<LabPhase>) {
        self.phase_filter = phase;
    }

    pub fn clear_filters(&mut self) {
        self.element_filter = None;
        self.pair_filter = None;
        self.phase_filter = None;
    }

    pub fn filtered_contacts(&self) -> Vec<&ContactSnapshot> {
        self.artifacts
            .final_snapshot
            .contacts
            .iter()
            .filter(|contact| {
                self.element_filter
                    .is_none_or(|id| contact.element_ids.contains(&id))
            })
            .filter(|contact| {
                self.pair_filter
                    .is_none_or(|pair| same_pair(contact.element_ids, pair))
            })
            .collect()
    }

    pub fn filtered_manifolds(&self) -> Vec<&ManifoldSnapshot> {
        self.artifacts
            .final_snapshot
            .manifolds
            .iter()
            .filter(|manifold| {
                self.element_filter
                    .is_none_or(|id| manifold.element_ids.contains(&id))
            })
            .filter(|manifold| {
                self.pair_filter
                    .is_none_or(|pair| same_pair(manifold.element_ids, pair))
            })
            .collect()
    }

    pub fn filtered_timeline(&self) -> Vec<&TraceEvent> {
        self.artifacts
            .trace_events
            .iter()
            .filter(|event| self.phase_filter.is_none_or(|phase| event.phase == phase))
            .filter(|event| {
                self.element_filter
                    .is_none_or(|id| event.element_ids.contains(&id))
            })
            .filter(|event| {
                self.pair_filter.is_none_or(|pair| {
                    event
                        .pair_id
                        .is_some_and(|event_pair| same_pair(event_pair, pair))
                })
            })
            .collect()
    }

    pub fn visible_shapes(&self) -> Vec<&DebugShape> {
        self.artifacts
            .debug_render
            .shapes
            .iter()
            .filter(|shape| {
                if let Some(pair) = self.pair_filter {
                    pair.contains(&shape.element_id)
                } else {
                    self.element_filter.is_none_or(|id| shape.element_id == id)
                }
            })
            .collect()
    }

    pub fn visible_manifold_labels(&self) -> Vec<&DebugLabel> {
        self.artifacts
            .debug_render
            .manifold_labels
            .iter()
            .filter(|label| {
                self.element_filter
                    .is_none_or(|id| label.element_ids.contains(&id))
            })
            .filter(|label| {
                self.pair_filter
                    .is_none_or(|pair| same_pair(label.element_ids, pair))
            })
            .collect()
    }

    pub fn select_at_world(&mut self, point: LabPoint) -> Option<SelectionTarget> {
        if let Some(contact) = self
            .artifacts
            .final_snapshot
            .contacts
            .iter()
            .find(|contact| distance_sq(contact.point, point) <= 0.25)
        {
            self.element_filter = None;
            self.pair_filter = Some(contact.element_ids);
            return Some(SelectionTarget::Pair(contact.element_ids));
        }

        if let Some(shape) = self
            .artifacts
            .debug_render
            .shapes
            .iter()
            .find(|shape| point_hits_shape(point, shape))
        {
            self.element_filter = Some(shape.element_id);
            self.pair_filter = None;
            return Some(SelectionTarget::Element(shape.element_id));
        }

        None
    }

    pub fn verification_markdown(&self) -> String {
        let summary = self.summary();
        let mut markdown = String::new();
        markdown.push_str("# Picea Lab Verification\n\n");
        markdown.push_str(&format!("run_id: {}\n", summary.run_id));
        markdown.push_str(&format!("state_hash: {}\n", summary.state_hash));
        markdown.push_str(&format!("elements: {}\n", summary.element_count));
        markdown.push_str(&format!("active_pairs: {}\n", summary.active_pair_count));
        markdown.push_str(&format!("contacts: {}\n", summary.contact_count));
        markdown.push_str(&format!(
            "timeline_events: {}\n\n",
            summary.timeline_event_count
        ));

        markdown.push_str("## Filters\n\n");
        markdown.push_str(&format!("element_filter: {:?}\n", self.element_filter));
        markdown.push_str(&format!("pair_filter: {:?}\n", self.pair_filter));
        markdown.push_str(&format!("phase_filter: {:?}\n\n", self.phase_filter));

        markdown.push_str("## Contacts\n\n");
        for contact in self.filtered_contacts() {
            markdown.push_str(&format!(
                "- pair={:?} contact_id={} depth={:.6}\n",
                contact.element_ids, contact.contact_id, contact.depth
            ));
        }

        markdown.push_str("\n## Manifolds\n\n");
        for manifold in self.filtered_manifolds() {
            markdown.push_str(&format!(
                "- pair={:?} active={} contact_points={}\n",
                manifold.element_ids, manifold.is_active, manifold.contact_point_count
            ));
        }

        markdown.push_str("\n## Timeline\n\n");
        for event in self.filtered_timeline().iter().take(20) {
            markdown.push_str(&format!(
                "- tick={} substep={:?} phase={:?} kind={:?}\n",
                event.tick, event.substep, event.phase, event.event_kind
            ));
        }

        markdown
    }

    pub fn export_verification_markdown(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        fs::write(path, self.verification_markdown())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionTarget {
    Element(ID),
    Pair([ID; 2]),
}

pub fn export_verification(
    artifact_dir: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let model = ViewerModel::load_from_dir(artifact_dir)?;
    model.export_verification_markdown(output_path)
}

pub fn run_viewer(artifact_dir: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let model = ViewerModel::load_from_dir(artifact_dir)?;
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Picea Lab",
        native_options,
        Box::new(move |_cc| Ok(Box::new(PiceaLabApp::new(model)))),
    )?;
    Ok(())
}

struct PiceaLabApp {
    model: ViewerModel,
    run_id_text: String,
    steps_text: String,
    second_circle_x_text: String,
    scenario_index: usize,
    element_filter_text: String,
    pair_filter_text: String,
    status_message: String,
}

impl PiceaLabApp {
    fn new(model: ViewerModel) -> Self {
        let (run_id_text, steps_text, second_circle_x_text, scenario_index) = match model.recipe() {
            RunRecipe::ContactReplay {
                run_id,
                second_circle_x,
                steps,
            } => (
                run_id.clone(),
                steps.to_string(),
                second_circle_x.to_string(),
                0,
            ),
            RunRecipe::Benchmark {
                run_id,
                scenario,
                steps,
            } => (
                run_id.clone(),
                steps.to_string(),
                String::new(),
                match scenario {
                    BenchmarkScenario::ContactRefreshTransfer => 1,
                    BenchmarkScenario::Circles32 => 2,
                },
            ),
        };

        Self {
            model,
            run_id_text,
            steps_text,
            second_circle_x_text,
            scenario_index,
            element_filter_text: String::new(),
            pair_filter_text: String::new(),
            status_message: String::new(),
        }
    }
}

impl eframe::App for PiceaLabApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(300.);
                ui.heading("Picea Lab");
                let summary = self.model.summary();
                ui.label(format!("run {}", summary.run_id));
                ui.monospace(format!("hash {}", summary.state_hash));
                ui.separator();
                ui.label(format!("elements {}", summary.element_count));
                ui.label(format!("contacts {}", summary.contact_count));
                ui.label(format!("events {}", summary.timeline_event_count));

                ui.separator();
                ui.heading("recipe");
                ui.label("run id");
                ui.text_edit_singleline(&mut self.run_id_text);
                ui.label("steps");
                ui.text_edit_singleline(&mut self.steps_text);
                ui.label("mode");
                egui::ComboBox::from_id_salt("recipe-mode")
                    .selected_text(match self.scenario_index {
                        0 => "contact replay",
                        1 => "benchmark/contact_refresh_transfer",
                        _ => "benchmark/circles_32",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.scenario_index, 0, "contact replay");
                        ui.selectable_value(
                            &mut self.scenario_index,
                            1,
                            "benchmark/contact_refresh_transfer",
                        );
                        ui.selectable_value(&mut self.scenario_index, 2, "benchmark/circles_32");
                    });
                if self.scenario_index == 0 {
                    ui.label("second circle x");
                    ui.text_edit_singleline(&mut self.second_circle_x_text);
                }
                if ui.button("regenerate").clicked() {
                    match build_recipe_from_controls(
                        &self.run_id_text,
                        &self.steps_text,
                        &self.second_circle_x_text,
                        self.scenario_index,
                    ) {
                        Ok(recipe) => {
                            self.model.set_recipe(recipe);
                            if let Err(error) = self.model.regenerate() {
                                self.status_message = error.to_string();
                            } else {
                                self.status_message = "regenerated".to_owned();
                            }
                        }
                        Err(error) => self.status_message = error,
                    }
                }
                if !self.status_message.is_empty() {
                    ui.monospace(&self.status_message);
                }

                ui.separator();
                ui.heading("render");
                ui.checkbox(
                    &mut self.model.render_settings_mut().show_contacts,
                    "show contacts",
                );
                ui.checkbox(
                    &mut self.model.render_settings_mut().show_normals,
                    "show normals",
                );
                ui.checkbox(
                    &mut self.model.render_settings_mut().show_manifold_labels,
                    "show manifold labels",
                );
                ui.checkbox(
                    &mut self.model.render_settings_mut().show_overlay_text,
                    "show overlay",
                );
                ui.add(
                    egui::Slider::new(&mut self.model.render_settings_mut().contact_radius, 2.0..=12.0)
                        .text("contact radius"),
                );
                ui.add(
                    egui::Slider::new(&mut self.model.render_settings_mut().normal_scale, 8.0..=96.0)
                        .text("normal scale"),
                );
                ui.add(
                    egui::Slider::new(&mut self.model.render_settings_mut().zoom, 0.5..=2.5)
                        .text("zoom"),
                );
                ui.add(
                    egui::Slider::new(&mut self.model.render_settings_mut().pan_x, -200.0..=200.0)
                        .text("pan x"),
                );
                ui.add(
                    egui::Slider::new(&mut self.model.render_settings_mut().pan_y, -200.0..=200.0)
                        .text("pan y"),
                );
                if ui.button("reset view").clicked() {
                    self.model.reset_view();
                }

                ui.separator();
                ui.heading("filters");
                ui.label("element id");
                if ui
                    .text_edit_singleline(&mut self.element_filter_text)
                    .changed()
                {
                    self.model
                        .set_element_filter(self.element_filter_text.parse::<ID>().ok());
                }

                ui.label("pair id, example 1,2");
                if ui
                    .text_edit_singleline(&mut self.pair_filter_text)
                    .changed()
                {
                    self.model
                        .set_pair_filter(parse_pair_filter(&self.pair_filter_text));
                }

                if ui.button("clear filters").clicked() {
                    self.element_filter_text.clear();
                    self.pair_filter_text.clear();
                    self.model.clear_filters();
                }

                ui.separator();
                ui.label("phase");
                for phase in PHASES {
                    let selected = self.model.phase_filter() == Some(phase);
                    if ui
                        .selectable_label(selected, format!("{:?}", phase))
                        .clicked()
                    {
                        if selected {
                            self.model.set_phase_filter(None);
                        } else {
                            self.model.set_phase_filter(Some(phase));
                        }
                    }
                }
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.heading("Viewport");
                if let Some(message) = draw_viewport(ui, &mut self.model) {
                    self.status_message = message;
                }

                ui.separator();
                ui.heading("Manifolds");
                egui::ScrollArea::vertical()
                    .id_salt("manifolds")
                    .max_height(110.)
                    .show(ui, |ui| {
                        for manifold in self.model.filtered_manifolds() {
                            ui.monospace(format!(
                                "pair {:?} active {} points {}",
                                manifold.element_ids,
                                manifold.is_active,
                                manifold.contact_point_count
                            ));
                        }
                    });

                ui.separator();
                ui.heading("Contacts");
                egui::ScrollArea::vertical()
                    .id_salt("contacts")
                    .max_height(140.)
                    .show(ui, |ui| {
                        for contact in self.model.filtered_contacts() {
                            ui.monospace(format!(
                                "pair {:?} id {} depth {:.4}",
                                contact.element_ids, contact.contact_id, contact.depth
                            ));
                        }
                    });

                ui.separator();
                ui.heading("Timeline");
                egui::ScrollArea::vertical()
                    .id_salt("timeline")
                    .max_height(220.)
                    .show(ui, |ui| {
                        for event in self.model.filtered_timeline() {
                            ui.monospace(format!(
                                "tick {} substep {:?} {:?} {:?}",
                                event.tick, event.substep, event.phase, event.event_kind
                            ));
                        }
                    });
            });
        });
    }
}

const PHASES: [LabPhase; 14] = [
    LabPhase::SceneTick,
    LabPhase::IntegrateVelocity,
    LabPhase::CollisionDetect,
    LabPhase::WarmStart,
    LabPhase::ContactRefresh,
    LabPhase::PreSolve,
    LabPhase::VelocitySolve,
    LabPhase::PositionIntegrate,
    LabPhase::PositionFix,
    LabPhase::SleepCheck,
    LabPhase::TransformSync,
    LabPhase::PostSolve,
    LabPhase::DebugRender,
    LabPhase::Perf,
];

fn draw_viewport(ui: &mut egui::Ui, model: &mut ViewerModel) -> Option<String> {
    let desired_size = Vec2::new(ui.available_width(), 360.);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 0., Color32::from_rgb(245, 247, 250));

    let Some(bounds) = model.artifacts().debug_render.world_bounds.clone() else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "empty world",
            egui::FontId::monospace(16.0),
            Color32::DARK_GRAY,
        );
        return None;
    };

    if response.hovered() {
        let scroll_y = ui.ctx().input(|input| input.smooth_scroll_delta.y);
        if scroll_y.abs() > f32::EPSILON {
            let factor = if scroll_y > 0.0 { 1.08 } else { 1.0 / 1.08 };
            model.zoom_view(factor);
        }
    }

    if response.dragged() {
        let delta = ui.ctx().input(|input| input.pointer.delta());
        model.pan_view(delta.x, delta.y);
    }

    let mut status = None;
    if response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let world_point =
                screen_to_world(rect, bounds.min, bounds.max, model.render_settings(), pointer);
            status = model.select_at_world(world_point).map(|selection| match selection {
                SelectionTarget::Element(id) => format!("selected element {id}"),
                SelectionTarget::Pair(pair) => format!("selected pair {}-{}", pair[0], pair[1]),
            });
        }
    }

    let artifacts = model.artifacts();
    let settings = model.render_settings();

    for shape in model.visible_shapes() {
        draw_shape(&painter, rect, bounds.min, bounds.max, settings, shape);
    }

    if settings.show_contacts {
        for contact in model.filtered_contacts() {
            let point = world_to_screen(rect, bounds.min, bounds.max, settings, contact.point);
            painter.circle_filled(point, settings.contact_radius, Color32::from_rgb(210, 64, 64));
            if settings.show_normals {
                let normal_end = Pos2::new(
                    point.x + contact.normal_toward_a.x * settings.normal_scale,
                    point.y - contact.normal_toward_a.y * settings.normal_scale,
                );
                painter.line_segment(
                    [point, normal_end],
                    Stroke::new(2.0, Color32::from_rgb(52, 116, 201)),
                );
            }
        }
    }

    if settings.show_manifold_labels {
        for label in model.visible_manifold_labels() {
            let related_contact = model
                .filtered_contacts()
                .into_iter()
                .find(|contact| same_pair(contact.element_ids, label.element_ids));
            let Some(contact) = related_contact else {
                continue;
            };
            let point = world_to_screen(rect, bounds.min, bounds.max, settings, contact.point);
            painter.text(
                Pos2::new(point.x + 8.0, point.y - 8.0),
                egui::Align2::LEFT_BOTTOM,
                &label.text,
                egui::FontId::monospace(12.0),
                Color32::from_rgb(31, 41, 55),
            );
        }
    }

    if settings.show_overlay_text {
        let width = 320.0;
        let height = (artifacts.debug_render.overlay_text.len().max(1) as f32 * 18.0) + 12.0;
        let overlay_rect =
            Rect::from_min_size(rect.left_top() + Vec2::new(12.0, 12.0), Vec2::new(width, height));
        painter.rect_filled(overlay_rect, 4.0, Color32::from_white_alpha(224));
        for (index, line) in artifacts.debug_render.overlay_text.iter().enumerate() {
            painter.text(
                overlay_rect.left_top() + Vec2::new(12.0, 12.0 + index as f32 * 18.0),
                egui::Align2::LEFT_TOP,
                line,
                egui::FontId::monospace(12.0),
                Color32::from_rgb(31, 41, 55),
            );
        }
    }

    status
}

fn draw_shape(
    painter: &egui::Painter,
    rect: Rect,
    min: LabPoint,
    max: LabPoint,
    settings: &RenderSettings,
    shape: &DebugShape,
) {
    for edge in &shape.edges {
        match edge {
            DebugEdge::Line { start, end } => {
                painter.line_segment(
                    [
                        world_to_screen(rect, min, max, settings, *start),
                        world_to_screen(rect, min, max, settings, *end),
                    ],
                    Stroke::new(1.5, Color32::from_rgb(31, 41, 55)),
                );
            }
            DebugEdge::Arc {
                start,
                support,
                end,
            } => {
                painter.line_segment(
                    [
                        world_to_screen(rect, min, max, settings, *start),
                        world_to_screen(rect, min, max, settings, *support),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(75, 85, 99)),
                );
                painter.line_segment(
                    [
                        world_to_screen(rect, min, max, settings, *support),
                        world_to_screen(rect, min, max, settings, *end),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(75, 85, 99)),
                );
            }
            DebugEdge::Circle { center, radius } => {
                let center = world_to_screen(rect, min, max, settings, *center);
                let scale = viewport_scale(rect, min, max, settings);
                painter.circle_stroke(
                    center,
                    *radius * scale,
                    Stroke::new(1.5, Color32::from_rgb(31, 41, 55)),
                );
            }
        }
    }
}

fn world_to_screen(
    rect: Rect,
    min: LabPoint,
    max: LabPoint,
    settings: &RenderSettings,
    point: LabPoint,
) -> Pos2 {
    let scale = viewport_scale(rect, min, max, settings);
    let world_width = (max.x - min.x).abs().max(1.0);
    let world_height = (max.y - min.y).abs().max(1.0);
    let offset_x = (rect.width() - world_width * scale) * 0.5;
    let offset_y = (rect.height() - world_height * scale) * 0.5;

    Pos2::new(
        rect.left() + offset_x + (point.x - min.x) * scale + settings.pan_x,
        rect.bottom() - offset_y - (point.y - min.y) * scale + settings.pan_y,
    )
}

fn screen_to_world(
    rect: Rect,
    min: LabPoint,
    max: LabPoint,
    settings: &RenderSettings,
    point: Pos2,
) -> LabPoint {
    let scale = viewport_scale(rect, min, max, settings);
    let world_width = (max.x - min.x).abs().max(1.0);
    let world_height = (max.y - min.y).abs().max(1.0);
    let offset_x = (rect.width() - world_width * scale) * 0.5;
    let offset_y = (rect.height() - world_height * scale) * 0.5;

    LabPoint {
        x: ((point.x - rect.left() - offset_x - settings.pan_x) / scale) + min.x,
        y: ((rect.bottom() - point.y - offset_y + settings.pan_y) / scale) + min.y,
    }
}

fn viewport_scale(rect: Rect, min: LabPoint, max: LabPoint, settings: &RenderSettings) -> f32 {
    let world_width = (max.x - min.x).abs().max(1.0);
    let world_height = (max.y - min.y).abs().max(1.0);
    (rect.width() / world_width).min(rect.height() / world_height) * 0.82 * settings.zoom
}

fn same_pair(left: [ID; 2], right: [ID; 2]) -> bool {
    left == right || left == [right[1], right[0]]
}

fn parse_pair_filter(raw: &str) -> Option<[ID; 2]> {
    let (left, right) = raw.split_once(',')?;
    Some([left.trim().parse().ok()?, right.trim().parse().ok()?])
}

fn distance_sq(left: LabPoint, right: LabPoint) -> f32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    (dx * dx) + (dy * dy)
}

fn point_hits_shape(point: LabPoint, shape: &DebugShape) -> bool {
    let radius = shape
        .edges
        .iter()
        .find_map(|edge| match edge {
            DebugEdge::Circle { radius, .. } => Some(*radius),
            _ => None,
        })
        .unwrap_or(1.0);

    distance_sq(point, shape.center) <= (radius * radius * 1.2)
}

fn infer_recipe_from_artifacts(artifacts: &LabArtifacts) -> RunRecipe {
    if artifacts.final_snapshot.elements.len() > 2 {
        RunRecipe::Benchmark {
            run_id: artifacts.final_snapshot.run_id.clone(),
            scenario: BenchmarkScenario::Circles32,
            steps: artifacts.final_snapshot.tick.max(1) as usize,
        }
    } else {
        RunRecipe::ContactReplay {
            run_id: artifacts.final_snapshot.run_id.clone(),
            second_circle_x: 1.5,
            steps: artifacts.final_snapshot.tick.max(1) as usize,
        }
    }
}

fn build_recipe_from_controls(
    run_id: &str,
    steps: &str,
    second_circle_x: &str,
    scenario_index: usize,
) -> Result<RunRecipe, String> {
    let steps = steps
        .trim()
        .parse::<usize>()
        .map_err(|_| "steps must be a positive integer".to_owned())?;
    if steps == 0 {
        return Err("steps must be greater than 0".to_owned());
    }

    match scenario_index {
        0 => {
            let second_circle_x = second_circle_x
                .trim()
                .parse::<f32>()
                .map_err(|_| "second circle x must be a finite number".to_owned())?;
            Ok(RunRecipe::ContactReplay {
                run_id: run_id.to_owned(),
                second_circle_x,
                steps,
            })
        }
        1 => Ok(RunRecipe::Benchmark {
            run_id: run_id.to_owned(),
            scenario: BenchmarkScenario::ContactRefreshTransfer,
            steps,
        }),
        _ => Ok(RunRecipe::Benchmark {
            run_id: run_id.to_owned(),
            scenario: BenchmarkScenario::Circles32,
            steps,
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::recipes::{capture_recipe, BenchmarkScenario, RunRecipe};

    #[test]
    fn viewer_model_inspects_filters_and_exports_verification_summary() {
        let artifacts = capture_recipe(RunRecipe::ContactReplay {
            run_id: "viewer".to_owned(),
            second_circle_x: 1.5,
            steps: 3,
        });

        let mut model = super::ViewerModel::from_artifacts(artifacts);
        let summary = model.summary();
        assert_eq!(summary.run_id, "viewer");
        assert_eq!(summary.element_count, 2);
        assert!(summary.contact_count > 0);
        assert!(!model.filtered_manifolds().is_empty());
        assert!(summary.timeline_event_count > 0);

        model.set_element_filter(Some(1));
        assert!(model
            .filtered_contacts()
            .iter()
            .all(|contact| contact.element_ids.contains(&1)));

        model.set_phase_filter(Some(picea::tools::observability::LabPhase::CollisionDetect));
        assert!(model
            .filtered_timeline()
            .iter()
            .all(|event| event.phase == picea::tools::observability::LabPhase::CollisionDetect));

        let markdown = model.verification_markdown();
        assert!(markdown.contains("# Picea Lab Verification"));
        assert!(markdown.contains("run_id: viewer"));
        assert!(markdown.contains("state_hash:"));
        assert!(markdown.contains("## Manifolds"));
    }

    #[test]
    fn viewer_model_can_regenerate_after_parameter_change() {
        let artifacts = capture_recipe(RunRecipe::ContactReplay {
            run_id: "interactive".to_owned(),
            second_circle_x: 1.5,
            steps: 3,
        });
        let mut model = super::ViewerModel::from_artifacts(artifacts);

        model.set_recipe(RunRecipe::ContactReplay {
            run_id: "interactive-adjusted".to_owned(),
            second_circle_x: 4.0,
            steps: 3,
        });
        model.regenerate().expect("recipe regenerates");

        assert_eq!(model.summary().run_id, "interactive-adjusted");
        assert_eq!(model.summary().contact_count, 0);

        model.set_recipe(RunRecipe::Benchmark {
            run_id: "bench".to_owned(),
            scenario: BenchmarkScenario::Circles32,
            steps: 2,
        });
        model.regenerate().expect("benchmark recipe regenerates");
        assert_eq!(model.summary().run_id, "bench");
        assert_eq!(model.summary().element_count, 32);
    }

    #[test]
    fn viewer_model_exposes_render_settings_for_contact_and_normal_visuals() {
        let artifacts = capture_recipe(RunRecipe::ContactReplay {
            run_id: "render".to_owned(),
            second_circle_x: 1.5,
            steps: 1,
        });
        let mut model = super::ViewerModel::from_artifacts(artifacts);

        assert!(model.render_settings().show_contacts);
        assert!(model.render_settings().show_normals);
        assert!(model.render_settings().show_manifold_labels);

        model.render_settings_mut().show_contacts = false;
        model.render_settings_mut().show_normals = false;
        model.render_settings_mut().normal_scale = 48.0;
        model.render_settings_mut().contact_radius = 7.0;

        assert!(!model.render_settings().show_contacts);
        assert!(!model.render_settings().show_normals);
        assert_eq!(model.render_settings().normal_scale, 48.0);
        assert_eq!(model.render_settings().contact_radius, 7.0);
    }

    #[test]
    fn viewer_model_supports_basic_view_navigation_and_selection() {
        let artifacts = capture_recipe(RunRecipe::ContactReplay {
            run_id: "interactive-nav".to_owned(),
            second_circle_x: 1.5,
            steps: 1,
        });
        let mut model = super::ViewerModel::from_artifacts(artifacts);

        model.zoom_view(10.0);
        model.pan_view(24.0, -16.0);
        assert_eq!(model.render_settings().zoom, 3.0);
        assert_eq!(model.render_settings().pan_x, 24.0);
        assert_eq!(model.render_settings().pan_y, -16.0);

        let selection = model.select_at_world(picea::tools::observability::LabPoint {
            x: 0.75,
            y: 0.0,
        });
        assert_eq!(selection, Some(super::SelectionTarget::Pair([1, 2])));
        assert_eq!(model.filtered_contacts().len(), 1);

        model.clear_filters();
        let selection = model.select_at_world(picea::tools::observability::LabPoint {
            x: 0.0,
            y: 0.0,
        });
        assert_eq!(selection, Some(super::SelectionTarget::Element(1)));
        assert!(model
            .visible_shapes()
            .iter()
            .all(|shape| shape.element_id == 1));

        model.reset_view();
        assert_eq!(model.render_settings().zoom, 1.0);
        assert_eq!(model.render_settings().pan_x, 0.0);
        assert_eq!(model.render_settings().pan_y, 0.0);
    }
}
