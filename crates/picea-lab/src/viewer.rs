use std::{error::Error, fs, path::Path};

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use picea::{
    element::ID,
    math::FloatNum,
    scene::Scene,
    tools::observability::{
        capture_scene_artifacts, ContactSnapshot, DebugEdge, DebugLabel, DebugShape, LabArtifacts,
        LabPhase, LabPoint, ManifoldSnapshot, TraceEvent, WorldBounds,
    },
};

#[cfg(test)]
use crate::recipes::capture_recipe;
use crate::{
    recipes::{build_scene, BenchmarkScenario, RunRecipe, STEP_DT},
    scene_spec::{ObjectShape, ObjectSpec, SceneTemplate, TemplateObjectId, WorldSpec},
};

pub struct ViewerModel {
    artifacts: LabArtifacts,
    scene: Scene<()>,
    run_id: String,
    recipe: RunRecipe,
    session: ViewerSession,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportTool {
    Select,
    AddCircle,
    AddBox,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewerSession {
    template: SceneTemplate,
    selected_object_id: Option<TemplateObjectId>,
    pending_tool: ViewportTool,
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

impl ViewerSession {
    pub fn new(template: SceneTemplate) -> Self {
        Self {
            template,
            selected_object_id: None,
            pending_tool: ViewportTool::Select,
        }
    }

    pub fn template(&self) -> &SceneTemplate {
        &self.template
    }

    pub fn template_mut(&mut self) -> &mut SceneTemplate {
        &mut self.template
    }

    pub fn selected_object_id(&self) -> Option<TemplateObjectId> {
        self.selected_object_id
    }

    pub fn selected_object(&self) -> Option<&ObjectSpec> {
        let selected_id = self.selected_object_id?;
        self.template
            .objects
            .iter()
            .find(|object| object.id == selected_id)
    }

    pub fn selected_object_mut(&mut self) -> Option<&mut ObjectSpec> {
        let selected_id = self.selected_object_id?;
        self.template
            .objects
            .iter_mut()
            .find(|object| object.id == selected_id)
    }

    pub fn pending_tool(&self) -> ViewportTool {
        self.pending_tool
    }

    #[cfg(test)]
    pub fn set_pending_tool(&mut self, tool: ViewportTool) {
        self.pending_tool = tool;
    }

    pub fn select_object(&mut self, id: Option<TemplateObjectId>) -> bool {
        if id.is_some()
            && !self
                .template
                .objects
                .iter()
                .any(|object| Some(object.id) == id)
        {
            return false;
        }
        self.selected_object_id = id;
        true
    }

    pub fn handle_primary_click(&mut self, point: [FloatNum; 2]) -> Option<String> {
        match self.pending_tool {
            ViewportTool::Select => {
                self.selected_object_id = self.hit_test(point);
                self.selected_object_id
                    .map(|id| format!("selected object {id}"))
                    .or_else(|| Some("selection cleared".to_owned()))
            }
            ViewportTool::AddCircle => {
                self.place_object(ObjectShape::default_circle(&self.template.world), point)
            }
            ViewportTool::AddBox => {
                self.place_object(ObjectShape::default_box(&self.template.world), point)
            }
        }
    }

    pub fn drag_selected_to(&mut self, point: [FloatNum; 2]) -> bool {
        let Some(selected_id) = self.selected_object_id else {
            return false;
        };
        let clamp = self.template.world.editor_clamp;
        let world = self.template.world.clone();
        if let Some(object) = self
            .template
            .objects
            .iter_mut()
            .find(|object| object.id == selected_id)
        {
            *object = object.with_position(point, &world, clamp);
            return true;
        }
        false
    }

    pub fn delete_selected(&mut self) -> bool {
        let Some(selected_id) = self.selected_object_id else {
            return false;
        };
        let before_len = self.template.objects.len();
        self.template
            .objects
            .retain(|object| object.id != selected_id);
        self.selected_object_id = None;
        before_len != self.template.objects.len()
    }

    pub fn update_selected_object_from_inspector(
        &mut self,
        position: [FloatNum; 2],
        velocity: [FloatNum; 2],
        is_fixed: bool,
        shape: ObjectShape,
    ) -> bool {
        let clamp = self.template.world.editor_clamp;
        let world = self.template.world.clone();
        if let Some(object) = self.selected_object_mut() {
            object.position = position;
            object.velocity = velocity;
            object.is_fixed = is_fixed;
            object.shape = shape;
            *object = object.normalized_for_world(&world, clamp);
            return true;
        }
        false
    }

    fn place_object(&mut self, shape: ObjectShape, point: [FloatNum; 2]) -> Option<String> {
        if let Some(existing_id) = self.hit_test(point) {
            self.selected_object_id = Some(existing_id);
            return Some(format!("selected object {existing_id}"));
        }

        let id = self.template.next_object_id();
        let position = if self.template.world.editor_clamp {
            self.template.world.clamp_position(&shape, point)
        } else {
            point
        };
        self.template.objects.push(ObjectSpec {
            id,
            position,
            velocity: [0.0, 0.0],
            is_fixed: false,
            shape,
        });
        self.selected_object_id = Some(id);
        Some(format!("placed object {id}"))
    }

    fn hit_test(&self, point: [FloatNum; 2]) -> Option<TemplateObjectId> {
        self.template
            .objects
            .iter()
            .rev()
            .find(|object| object.contains_point(point))
            .map(|object| object.id)
    }
}

impl ViewerModel {
    pub fn from_artifacts(artifacts: LabArtifacts) -> Self {
        let run_id = artifacts.final_snapshot.run_id.clone();
        let session = ViewerSession::new(infer_template_from_artifacts(&artifacts));
        Self {
            recipe: infer_recipe_from_artifacts(&artifacts),
            scene: build_scene(session.template()),
            run_id,
            session,
            render_settings: RenderSettings::default(),
            artifacts,
            element_filter: None,
            pair_filter: None,
            phase_filter: None,
        }
    }

    #[cfg(test)]
    pub fn from_template(run_id: impl Into<String>, template: SceneTemplate) -> Self {
        let run_id = run_id.into();
        let scene = build_scene(&template);
        let artifacts = capture_scene_artifacts(run_id.clone(), &scene);
        Self {
            recipe: infer_recipe_from_artifacts(&artifacts),
            scene,
            run_id,
            session: ViewerSession::new(template),
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

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn set_run_id(&mut self, run_id: impl Into<String>) {
        self.run_id = run_id.into();
    }

    pub fn session(&self) -> &ViewerSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut ViewerSession {
        &mut self.session
    }

    pub fn recipe_label(&self) -> &'static str {
        match self.recipe {
            RunRecipe::ContactReplay { .. } => "contact replay",
            RunRecipe::Benchmark {
                scenario: BenchmarkScenario::ContactRefreshTransfer,
                ..
            } => "benchmark / contact refresh",
            RunRecipe::Benchmark {
                scenario: BenchmarkScenario::Circles32,
                ..
            } => "benchmark / circles 32",
        }
    }

    #[cfg(test)]
    pub fn set_recipe(&mut self, recipe: RunRecipe) {
        self.run_id = recipe_run_id(&recipe).to_owned();
        self.recipe = recipe;
    }

    #[cfg(test)]
    pub fn regenerate(&mut self) -> std::io::Result<()> {
        self.artifacts = capture_recipe(self.recipe.clone());
        self.run_id = self.artifacts.final_snapshot.run_id.clone();
        self.session = ViewerSession::new(infer_template_from_artifacts(&self.artifacts));
        self.scene = build_scene(self.session.template());
        self.clear_filters();
        Ok(())
    }

    pub fn regenerate_from_template(&mut self) -> std::io::Result<()> {
        self.scene = build_scene(self.session.template());
        self.artifacts = capture_scene_artifacts(self.run_id.clone(), &self.scene);
        self.clear_filters();
        Ok(())
    }

    pub fn step_once(&mut self) {
        self.scene.tick(STEP_DT);
        self.artifacts = capture_scene_artifacts(self.run_id.clone(), &self.scene);
    }

    pub fn delete_selected_object(&mut self) -> Option<String> {
        if self.session.delete_selected() {
            if self.regenerate_from_template().is_ok() {
                Some("deleted selected object".to_owned())
            } else {
                Some("failed to regenerate after delete".to_owned())
            }
        } else {
            None
        }
    }

    pub fn handle_viewport_primary_click(&mut self, point: [FloatNum; 2]) -> Option<String> {
        let object_count_before = self.session.template().objects.len();
        let message = self.session.handle_primary_click(point);
        if self.session.template().objects.len() != object_count_before {
            match self.regenerate_from_template() {
                Ok(()) => message,
                Err(error) => Some(error.to_string()),
            }
        } else {
            message
        }
    }

    pub fn handle_drag_selected_object(&mut self, point: [FloatNum; 2]) -> Option<String> {
        if !self.session.drag_selected_to(point) {
            return None;
        }
        match self.regenerate_from_template() {
            Ok(()) => Some("moved selected object".to_owned()),
            Err(error) => Some(error.to_string()),
        }
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

    #[cfg(test)]
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

#[cfg(test)]
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
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Picea Lab")
            .with_inner_size([1400.0, 920.0])
            .with_min_inner_size([960.0, 640.0])
            .with_visible(true),
        centered: true,
        run_and_return: false,
        ..Default::default()
    };
    eframe::run_native(
        "Picea Lab",
        native_options,
        Box::new(move |cc| {
            apply_app_theme(&cc.egui_ctx);
            Ok(Box::new(PiceaLabApp::new(model)))
        }),
    )?;
    Ok(())
}

struct PiceaLabApp {
    model: ViewerModel,
    run_id_text: String,
    world_width_text: String,
    world_height_text: String,
    gravity_text: String,
    element_filter_text: String,
    pair_filter_text: String,
    status_message: String,
}

impl PiceaLabApp {
    fn new(model: ViewerModel) -> Self {
        let run_id_text = model.run_id().to_owned();
        let world = model.session().template().world.clone();

        Self {
            model,
            run_id_text,
            world_width_text: world.width.to_string(),
            world_height_text: world.height.to_string(),
            gravity_text: world.gravity.to_string(),
            element_filter_text: String::new(),
            pair_filter_text: String::new(),
            status_message: String::new(),
        }
    }

    fn sync_world_inputs_from_model(&mut self) {
        let world = self.model.session().template().world.clone();
        self.world_width_text = world.width.to_string();
        self.world_height_text = world.height.to_string();
        self.gravity_text = world.gravity.to_string();
    }

    fn apply_world_inputs(&mut self) -> Result<(), String> {
        let width = self
            .world_width_text
            .trim()
            .parse::<FloatNum>()
            .map_err(|_| "world width must be a finite number".to_owned())?;
        let height = self
            .world_height_text
            .trim()
            .parse::<FloatNum>()
            .map_err(|_| "world height must be a finite number".to_owned())?;
        let gravity = self
            .gravity_text
            .trim()
            .parse::<FloatNum>()
            .map_err(|_| "gravity must be a finite number".to_owned())?;

        if width <= 0.0 || height <= 0.0 {
            return Err("world width and height must be greater than 0".to_owned());
        }

        let session = self.model.session_mut();
        session.template_mut().world.width = width;
        session.template_mut().world.height = height;
        session.template_mut().world.gravity = gravity;

        if session.template().world.editor_clamp {
            let world = session.template().world.clone();
            for object in &mut session.template_mut().objects {
                *object = object.clamped_to(&world);
            }
        }

        self.model.set_run_id(self.run_id_text.trim().to_owned());
        Ok(())
    }
}

impl eframe::App for PiceaLabApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        apply_app_theme(ui.ctx());

        if self.run_id_text.is_empty() {
            self.run_id_text = self.model.run_id().to_owned();
        }

        if ui.ctx().input(|input| input.key_pressed(egui::Key::Delete)) {
            if let Some(message) = self.model.delete_selected_object() {
                self.status_message = message;
            }
        }

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(300.);
                ui.heading("Picea Lab");
                let summary = self.model.summary();
                ui.label(format!("run {}", summary.run_id));
                ui.monospace(self.model.recipe_label());
                ui.monospace(format!("hash {}", summary.state_hash));
                ui.separator();
                ui.label(format!("elements {}", summary.element_count));
                ui.label(format!("contacts {}", summary.contact_count));
                ui.label(format!("events {}", summary.timeline_event_count));

                ui.separator();
                section_frame().show(ui, |ui| {
                    ui.heading("Scene");
                    ui.label("run id");
                    ui.text_edit_singleline(&mut self.run_id_text);
                    ui.label("world width");
                    ui.text_edit_singleline(&mut self.world_width_text);
                    ui.label("world height");
                    ui.text_edit_singleline(&mut self.world_height_text);
                    ui.label("gravity");
                    ui.text_edit_singleline(&mut self.gravity_text);
                    ui.checkbox(
                        &mut self.model.session_mut().template_mut().world.editor_clamp,
                        "editor clamp",
                    );
                    ui.checkbox(
                        &mut self
                            .model
                            .session_mut()
                            .template_mut()
                            .world
                            .runtime_boundary,
                        "runtime boundary",
                    );

                    ui.horizontal_wrapped(|ui| {
                        if ui.button("regenerate").clicked() {
                            match self.apply_world_inputs() {
                                Ok(()) => {
                                    if let Err(error) = self.model.regenerate_from_template() {
                                        self.status_message = error.to_string();
                                    } else {
                                        self.sync_world_inputs_from_model();
                                        self.status_message =
                                            "regenerated from template".to_owned();
                                    }
                                }
                                Err(error) => {
                                    self.status_message = error;
                                }
                            }
                        }
                        if ui.button("step once").clicked() {
                            if self.apply_world_inputs().is_ok() {
                                self.model.step_once();
                                self.status_message = "stepped once".to_owned();
                            }
                        }
                        if ui.button("delete selected").clicked() {
                            if let Some(message) = self.model.delete_selected_object() {
                                self.status_message = message;
                            }
                        }
                    });
                });

                ui.add_space(6.0);
                section_frame().show(ui, |ui| {
                    ui.heading("Objects");
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(
                            &mut self.model.session_mut().pending_tool,
                            ViewportTool::Select,
                            "Select",
                        );
                        ui.selectable_value(
                            &mut self.model.session_mut().pending_tool,
                            ViewportTool::AddCircle,
                            "Add Circle",
                        );
                        ui.selectable_value(
                            &mut self.model.session_mut().pending_tool,
                            ViewportTool::AddBox,
                            "Add Box",
                        );
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("object-list")
                        .max_height(140.)
                        .show(ui, |ui| {
                            let rows = self
                                .model
                                .session()
                                .template()
                                .objects
                                .iter()
                                .map(|object| {
                                    (
                                        object.id,
                                        object_shape_name(&object.shape),
                                        object.position,
                                        self.model.session().selected_object_id()
                                            == Some(object.id),
                                    )
                                })
                                .collect::<Vec<_>>();
                            for (id, shape_name, position, is_selected) in rows {
                                if ui
                                    .selectable_label(
                                        is_selected,
                                        format!(
                                            "#{id} {shape_name} ({:.1}, {:.1})",
                                            position[0], position[1]
                                        ),
                                    )
                                    .clicked()
                                {
                                    let _ = self.model.session_mut().select_object(Some(id));
                                    self.status_message = format!("selected object {id}");
                                }
                            }
                        });
                    if let Some(selected_id) = self.model.session().selected_object_id() {
                        ui.monospace(format!("selected object {selected_id}"));
                    } else {
                        ui.monospace("selected object none");
                    }
                    ui.monospace(format!(
                        "tool {:?} objects {}",
                        self.model.session().pending_tool(),
                        self.model.session().template().objects.len()
                    ));
                });

                ui.add_space(6.0);
                section_frame().show(ui, |ui| {
                    ui.heading("Selection Inspector");
                    if let Some(selected) = self.model.session().selected_object().cloned() {
                        let mut position = selected.position;
                        let mut velocity = selected.velocity;
                        let mut is_fixed = selected.is_fixed;
                        let mut shape = selected.shape.clone();
                        let mut changed = false;

                        ui.monospace(format!(
                            "{} #{}",
                            object_shape_name(&selected.shape),
                            selected.id
                        ));

                        ui.horizontal(|ui| {
                            ui.label("x");
                            changed |= ui
                                .add(egui::DragValue::new(&mut position[0]).speed(0.25))
                                .changed();
                            ui.label("y");
                            changed |= ui
                                .add(egui::DragValue::new(&mut position[1]).speed(0.25))
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("vx");
                            changed |= ui
                                .add(egui::DragValue::new(&mut velocity[0]).speed(0.25))
                                .changed();
                            ui.label("vy");
                            changed |= ui
                                .add(egui::DragValue::new(&mut velocity[1]).speed(0.25))
                                .changed();
                        });
                        changed |= ui.checkbox(&mut is_fixed, "fixed body").changed();

                        match &mut shape {
                            ObjectShape::Circle { radius } => {
                                ui.horizontal(|ui| {
                                    ui.label("radius");
                                    changed |=
                                        ui.add(egui::DragValue::new(radius).speed(0.25)).changed();
                                });
                            }
                            ObjectShape::Box { width, height } => {
                                ui.horizontal(|ui| {
                                    ui.label("width");
                                    changed |=
                                        ui.add(egui::DragValue::new(width).speed(0.25)).changed();
                                    ui.label("height");
                                    changed |=
                                        ui.add(egui::DragValue::new(height).speed(0.25)).changed();
                                });
                            }
                        }

                        if changed {
                            let updated = self
                                .model
                                .session_mut()
                                .update_selected_object_from_inspector(
                                    position, velocity, is_fixed, shape,
                                );
                            if updated {
                                match self.model.regenerate_from_template() {
                                    Ok(()) => {
                                        self.status_message =
                                            format!("updated object {}", selected.id);
                                    }
                                    Err(error) => {
                                        self.status_message = error.to_string();
                                    }
                                }
                            }
                        }
                    } else {
                        ui.label("Select an object in the viewport or object list to edit it.");
                    }
                });

                if !self.status_message.is_empty() {
                    ui.add_space(6.0);
                    status_frame().show(ui, |ui| {
                        ui.colored_label(STATUS_OK_TEXT, &self.status_message);
                    });
                }

                ui.add_space(6.0);
                section_frame().show(ui, |ui| {
                    ui.heading("Render");
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
                        egui::Slider::new(
                            &mut self.model.render_settings_mut().contact_radius,
                            2.0..=12.0,
                        )
                        .text("contact radius"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.model.render_settings_mut().normal_scale,
                            8.0..=96.0,
                        )
                        .text("normal scale"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.model.render_settings_mut().zoom, 0.5..=2.5)
                            .text("zoom"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.model.render_settings_mut().pan_x,
                            -200.0..=200.0,
                        )
                        .text("pan x"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.model.render_settings_mut().pan_y,
                            -200.0..=200.0,
                        )
                        .text("pan y"),
                    );
                    if ui.button("reset view").clicked() {
                        self.model.reset_view();
                    }
                });

                ui.add_space(6.0);
                section_frame().show(ui, |ui| {
                    ui.heading("Filters");
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
            });

            ui.separator();

            ui.vertical(|ui| {
                section_frame().show(ui, |ui| {
                    ui.heading("Viewport");
                    ui.label(viewport_hint_text(
                        self.model.session().pending_tool(),
                        self.model.session().selected_object_id(),
                    ));
                    if let Some(message) = draw_viewport(ui, &mut self.model) {
                        self.status_message = message;
                    }
                });

                ui.add_space(6.0);
                section_frame().show(ui, |ui| {
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
                });

                ui.add_space(6.0);
                section_frame().show(ui, |ui| {
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
                });

                ui.add_space(6.0);
                section_frame().show(ui, |ui| {
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

const PANEL_BG: Color32 = Color32::from_rgb(244, 239, 229);
const PANEL_BG_ALT: Color32 = Color32::from_rgb(236, 229, 216);
const PANEL_BORDER: Color32 = Color32::from_rgb(183, 169, 145);
const PANEL_TEXT: Color32 = Color32::from_rgb(43, 38, 32);
const VIEWPORT_BG: Color32 = Color32::from_rgb(24, 30, 38);
const VIEWPORT_GRID_MAJOR: Color32 = Color32::from_rgba_premultiplied(115, 138, 166, 72);
const VIEWPORT_GRID_MINOR: Color32 = Color32::from_rgba_premultiplied(115, 138, 166, 28);
const VIEWPORT_WORLD_BORDER: Color32 = Color32::from_rgb(202, 174, 111);
const SHAPE_STROKE: Color32 = Color32::from_rgb(232, 236, 240);
const SELECTED_STROKE: Color32 = Color32::from_rgb(255, 196, 92);
const STATUS_OK_BG: Color32 = Color32::from_rgb(223, 243, 230);
const STATUS_OK_TEXT: Color32 = Color32::from_rgb(33, 84, 52);

fn object_shape_name(shape: &ObjectShape) -> &'static str {
    match shape {
        ObjectShape::Circle { .. } => "Circle",
        ObjectShape::Box { .. } => "Box",
    }
}

fn viewport_hint_text(tool: ViewportTool, selected: Option<TemplateObjectId>) -> String {
    match (tool, selected) {
        (ViewportTool::Select, Some(id)) => {
            format!("Selected object {id}. Drag it to move, or Drag empty space to pan.")
        }
        (ViewportTool::Select, None) => {
            "Select mode. Click an object to inspect it, or Drag empty space to pan.".to_owned()
        }
        (ViewportTool::AddCircle, Some(id)) => {
            format!("Circle tool active. Click empty space to place a circle. Current selection: object {id}.")
        }
        (ViewportTool::AddCircle, None) => {
            "Circle tool active. Click to place a circle.".to_owned()
        }
        (ViewportTool::AddBox, Some(id)) => {
            format!("Box tool active. Click empty space to place a box. Current selection: object {id}.")
        }
        (ViewportTool::AddBox, None) => "Box tool active. Click to place a box.".to_owned(),
    }
}

fn apply_app_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.window_fill = PANEL_BG;
    visuals.panel_fill = PANEL_BG;
    visuals.extreme_bg_color = PANEL_BG_ALT;
    visuals.faint_bg_color = PANEL_BG_ALT;
    visuals.widgets.noninteractive.bg_fill = PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, PANEL_BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, PANEL_TEXT);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(250, 247, 241);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, PANEL_BORDER);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(255, 251, 244);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(123, 109, 88));
    visuals.widgets.active.bg_fill = Color32::from_rgb(227, 214, 188);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2, Color32::from_rgb(96, 78, 48));
    visuals.selection.bg_fill = Color32::from_rgb(212, 170, 92);
    visuals.selection.stroke = Stroke::new(1.2, Color32::from_rgb(73, 53, 17));
    visuals.hyperlink_color = Color32::from_rgb(34, 94, 146);
    visuals.override_text_color = Some(PANEL_TEXT);
    visuals.window_stroke = Stroke::new(1.0, PANEL_BORDER);
    visuals.code_bg_color = PANEL_BG_ALT;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(10.0, 8.0);
    style.spacing.indent = 12.0;
    ctx.set_global_style(style);
}

fn section_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(PANEL_BG_ALT)
        .stroke(Stroke::new(1.0, PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(12, 10))
}

fn status_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(STATUS_OK_BG)
        .stroke(Stroke::new(1.0, Color32::from_rgb(167, 209, 183)))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
}

fn draw_viewport(ui: &mut egui::Ui, model: &mut ViewerModel) -> Option<String> {
    let desired_size = Vec2::new(ui.available_width(), 360.);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 12.0, VIEWPORT_BG);

    let bounds = viewport_bounds(model);
    draw_viewport_grid(&painter, rect, &bounds, model.render_settings());
    let world_min = world_to_screen(
        rect,
        bounds.min,
        bounds.max,
        model.render_settings(),
        bounds.min,
    );
    let world_max = world_to_screen(
        rect,
        bounds.min,
        bounds.max,
        model.render_settings(),
        bounds.max,
    );
    let world_rect = Rect::from_two_pos(
        Pos2::new(world_min.x, world_max.y),
        Pos2::new(world_max.x, world_min.y),
    );
    painter.rect_stroke(
        world_rect,
        10.0,
        Stroke::new(1.4, VIEWPORT_WORLD_BORDER),
        egui::StrokeKind::Middle,
    );

    if response.hovered() {
        let scroll_y = ui.ctx().input(|input| input.smooth_scroll_delta.y);
        if scroll_y.abs() > f32::EPSILON {
            let factor = if scroll_y > 0.0 { 1.08 } else { 1.0 / 1.08 };
            model.zoom_view(factor);
        }
    }

    let mut status = None;
    if let Some(pointer) = response.interact_pointer_pos() {
        let world_point = screen_to_world(
            rect,
            bounds.min,
            bounds.max,
            model.render_settings(),
            pointer,
        );

        if response.clicked() {
            status = model.handle_viewport_primary_click([world_point.x, world_point.y]);
        }

        if response.dragged() {
            if model.session().pending_tool() == ViewportTool::Select
                && model.session().selected_object_id().is_some()
            {
                status = model.handle_drag_selected_object([world_point.x, world_point.y]);
            } else {
                let delta = ui.ctx().input(|input| input.pointer.delta());
                model.pan_view(delta.x, delta.y);
            }
        }
    }

    let artifacts = model.artifacts();
    let settings = model.render_settings();

    for shape in model.visible_shapes() {
        let is_selected =
            model.session().selected_object_id() == Some(shape.element_id as TemplateObjectId);
        draw_shape(
            &painter,
            rect,
            bounds.min,
            bounds.max,
            settings,
            shape,
            is_selected,
        );
    }

    if settings.show_contacts {
        for contact in model.filtered_contacts() {
            let point = world_to_screen(rect, bounds.min, bounds.max, settings, contact.point);
            painter.circle_filled(
                point,
                settings.contact_radius,
                Color32::from_rgb(210, 64, 64),
            );
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
                Color32::from_rgb(247, 234, 203),
            );
        }
    }

    if settings.show_overlay_text {
        let width = 320.0;
        let height = (artifacts.debug_render.overlay_text.len().max(1) as f32 * 18.0) + 12.0;
        let overlay_rect = Rect::from_min_size(
            rect.left_top() + Vec2::new(12.0, 12.0),
            Vec2::new(width, height),
        );
        painter.rect_filled(
            overlay_rect,
            8.0,
            Color32::from_rgba_premultiplied(14, 19, 24, 216),
        );
        for (index, line) in artifacts.debug_render.overlay_text.iter().enumerate() {
            painter.text(
                overlay_rect.left_top() + Vec2::new(12.0, 12.0 + index as f32 * 18.0),
                egui::Align2::LEFT_TOP,
                line,
                egui::FontId::monospace(12.0),
                Color32::from_rgb(233, 238, 244),
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
    is_selected: bool,
) {
    let stroke = Stroke::new(
        if is_selected { 2.6 } else { 1.6 },
        if is_selected {
            SELECTED_STROKE
        } else {
            SHAPE_STROKE
        },
    );
    for edge in &shape.edges {
        match edge {
            DebugEdge::Line { start, end } => {
                painter.line_segment(
                    [
                        world_to_screen(rect, min, max, settings, *start),
                        world_to_screen(rect, min, max, settings, *end),
                    ],
                    stroke,
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
                    stroke,
                );
                painter.line_segment(
                    [
                        world_to_screen(rect, min, max, settings, *support),
                        world_to_screen(rect, min, max, settings, *end),
                    ],
                    stroke,
                );
            }
            DebugEdge::Circle { center, radius } => {
                let center = world_to_screen(rect, min, max, settings, *center);
                let scale = viewport_scale(rect, min, max, settings);
                painter.circle_stroke(center, *radius * scale, stroke);
            }
        }
    }

    if is_selected {
        let center = world_to_screen(rect, min, max, settings, shape.center);
        painter.circle_filled(center, 4.0, SELECTED_STROKE);
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

fn draw_viewport_grid(
    painter: &egui::Painter,
    rect: Rect,
    bounds: &WorldBounds,
    settings: &RenderSettings,
) {
    let major_step = grid_step(bounds);
    let minor_step = (major_step * 0.5).max(1.0);

    draw_grid_lines(
        painter,
        rect,
        bounds,
        settings,
        minor_step,
        Stroke::new(1.0, VIEWPORT_GRID_MINOR),
    );
    draw_grid_lines(
        painter,
        rect,
        bounds,
        settings,
        major_step,
        Stroke::new(1.0, VIEWPORT_GRID_MAJOR),
    );
}

fn draw_grid_lines(
    painter: &egui::Painter,
    rect: Rect,
    bounds: &WorldBounds,
    settings: &RenderSettings,
    step: FloatNum,
    stroke: Stroke,
) {
    let mut x = bounds.min.x;
    while x <= bounds.max.x {
        let top = world_to_screen(
            rect,
            bounds.min,
            bounds.max,
            settings,
            LabPoint { x, y: bounds.max.y },
        );
        let bottom = world_to_screen(
            rect,
            bounds.min,
            bounds.max,
            settings,
            LabPoint { x, y: bounds.min.y },
        );
        painter.line_segment([top, bottom], stroke);
        x += step;
    }

    let mut y = bounds.min.y;
    while y <= bounds.max.y {
        let left = world_to_screen(
            rect,
            bounds.min,
            bounds.max,
            settings,
            LabPoint { x: bounds.min.x, y },
        );
        let right = world_to_screen(
            rect,
            bounds.min,
            bounds.max,
            settings,
            LabPoint { x: bounds.max.x, y },
        );
        painter.line_segment([left, right], stroke);
        y += step;
    }
}

fn grid_step(bounds: &WorldBounds) -> FloatNum {
    let major = (bounds.max.x - bounds.min.x)
        .abs()
        .max((bounds.max.y - bounds.min.y).abs());
    if major <= 20.0 {
        2.0
    } else if major <= 60.0 {
        5.0
    } else if major <= 150.0 {
        10.0
    } else {
        20.0
    }
}

fn viewport_bounds(model: &ViewerModel) -> WorldBounds {
    model
        .artifacts()
        .debug_render
        .world_bounds
        .clone()
        .unwrap_or_else(|| model.session().template().world_bounds())
}

fn same_pair(left: [ID; 2], right: [ID; 2]) -> bool {
    left == right || left == [right[1], right[0]]
}

fn parse_pair_filter(raw: &str) -> Option<[ID; 2]> {
    let (left, right) = raw.split_once(',')?;
    Some([left.trim().parse().ok()?, right.trim().parse().ok()?])
}

#[cfg(test)]
fn distance_sq(left: LabPoint, right: LabPoint) -> f32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    (dx * dx) + (dy * dy)
}

#[cfg(test)]
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

fn infer_template_from_artifacts(artifacts: &LabArtifacts) -> SceneTemplate {
    let world = artifacts
        .debug_render
        .world_bounds
        .clone()
        .map(|bounds| WorldSpec {
            width: (bounds.max.x - bounds.min.x).abs().max(1.0),
            height: (bounds.max.y - bounds.min.y).abs().max(1.0),
            gravity: 0.0,
            editor_clamp: false,
            runtime_boundary: false,
        })
        .unwrap_or(WorldSpec {
            width: 100.0,
            height: 60.0,
            gravity: 0.0,
            editor_clamp: false,
            runtime_boundary: false,
        });

    let objects = artifacts
        .debug_render
        .shapes
        .iter()
        .map(|shape| infer_object_spec(artifacts, shape))
        .collect();

    SceneTemplate { world, objects }
}

fn infer_object_spec(artifacts: &LabArtifacts, shape: &DebugShape) -> ObjectSpec {
    let element = artifacts
        .final_snapshot
        .elements
        .iter()
        .find(|element| element.id == shape.element_id);
    let velocity = element
        .map(|element| [element.velocity.x, element.velocity.y])
        .unwrap_or([0.0, 0.0]);
    let is_fixed = element.is_some_and(|element| element.is_fixed);

    ObjectSpec {
        id: shape.element_id as TemplateObjectId,
        position: [shape.center.x, shape.center.y],
        velocity,
        is_fixed,
        shape: infer_shape_from_debug(shape),
    }
}

fn infer_shape_from_debug(shape: &DebugShape) -> ObjectShape {
    if let Some(radius) = shape.edges.iter().find_map(|edge| match edge {
        DebugEdge::Circle { radius, .. } => Some(*radius),
        _ => None,
    }) {
        return ObjectShape::Circle { radius };
    }

    let mut min_x = shape.center.x;
    let mut max_x = shape.center.x;
    let mut min_y = shape.center.y;
    let mut max_y = shape.center.y;
    for edge in &shape.edges {
        match edge {
            DebugEdge::Line { start, end } => {
                min_x = min_x.min(start.x).min(end.x);
                max_x = max_x.max(start.x).max(end.x);
                min_y = min_y.min(start.y).min(end.y);
                max_y = max_y.max(start.y).max(end.y);
            }
            DebugEdge::Arc {
                start,
                support,
                end,
            } => {
                for point in [start, support, end] {
                    min_x = min_x.min(point.x);
                    max_x = max_x.max(point.x);
                    min_y = min_y.min(point.y);
                    max_y = max_y.max(point.y);
                }
            }
            DebugEdge::Circle { center, radius } => {
                min_x = min_x.min(center.x - radius);
                max_x = max_x.max(center.x + radius);
                min_y = min_y.min(center.y - radius);
                max_y = max_y.max(center.y + radius);
            }
        }
    }

    ObjectShape::Box {
        width: (max_x - min_x).abs().max(1.0),
        height: (max_y - min_y).abs().max(1.0),
    }
}

#[cfg(test)]
fn recipe_run_id(recipe: &RunRecipe) -> &str {
    match recipe {
        RunRecipe::ContactReplay { run_id, .. } | RunRecipe::Benchmark { run_id, .. } => run_id,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        recipes::{capture_recipe, BenchmarkScenario, RunRecipe},
        scene_spec::{SceneTemplate, WorldSpec},
    };

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
    fn viewer_model_can_add_object_and_regenerate_from_scene_template() {
        let mut model = super::ViewerModel::from_template(
            "editor-template",
            SceneTemplate {
                world: WorldSpec {
                    width: 120.0,
                    height: 80.0,
                    gravity: 0.0,
                    editor_clamp: true,
                    runtime_boundary: false,
                },
                objects: Vec::new(),
            },
        );

        model
            .session_mut()
            .set_pending_tool(super::ViewportTool::AddCircle);
        model.session_mut().handle_primary_click([20.0, 20.0]);
        model
            .regenerate_from_template()
            .expect("template regenerate succeeds");

        assert_eq!(model.summary().element_count, 1);
        assert_eq!(model.session().template().objects.len(), 1);
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

        let selection =
            model.select_at_world(picea::tools::observability::LabPoint { x: 0.75, y: 0.0 });
        assert_eq!(selection, Some(super::SelectionTarget::Pair([1, 2])));
        assert_eq!(model.filtered_contacts().len(), 1);

        model.clear_filters();
        let selection =
            model.select_at_world(picea::tools::observability::LabPoint { x: 0.0, y: 0.0 });
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

    #[test]
    fn viewport_interaction_logic_places_selects_and_deletes_objects() {
        let mut session = super::ViewerSession::new(SceneTemplate {
            world: WorldSpec {
                width: 100.0,
                height: 60.0,
                gravity: 0.0,
                editor_clamp: true,
                runtime_boundary: false,
            },
            objects: Vec::new(),
        });

        session.set_pending_tool(super::ViewportTool::AddBox);
        session.handle_primary_click([32.0, 24.0]);

        assert_eq!(session.template().objects.len(), 1);
        let object_id = session
            .selected_object_id()
            .expect("newly placed object becomes selected");

        session.set_pending_tool(super::ViewportTool::Select);
        session.handle_primary_click([32.0, 24.0]);
        assert_eq!(session.selected_object_id(), Some(object_id));

        session.delete_selected();
        assert!(session.template().objects.is_empty());
        assert_eq!(session.selected_object_id(), None);
    }

    #[test]
    fn viewer_session_updates_selected_object_via_inspector_and_clamps_it() {
        let mut session = super::ViewerSession::new(SceneTemplate {
            world: WorldSpec {
                width: 30.0,
                height: 20.0,
                gravity: 0.0,
                editor_clamp: true,
                runtime_boundary: false,
            },
            objects: vec![crate::scene_spec::ObjectSpec {
                id: 1,
                position: [6.0, 6.0],
                velocity: [0.0, 0.0],
                is_fixed: false,
                shape: crate::scene_spec::ObjectShape::Circle { radius: 2.0 },
            }],
        });

        session.select_object(Some(1));
        let updated = session.update_selected_object_from_inspector(
            [50.0, -10.0],
            [3.0, -2.0],
            true,
            crate::scene_spec::ObjectShape::Circle { radius: -4.0 },
        );

        assert!(updated);
        let object = session.selected_object().expect("object remains selected");
        assert_eq!(object.velocity, [3.0, -2.0]);
        assert!(object.is_fixed);
        assert_eq!(
            object.shape,
            crate::scene_spec::ObjectShape::Circle { radius: 4.0 }
        );
        assert_eq!(object.position, [26.0, 4.0]);
    }

    #[test]
    fn viewport_hint_reflects_tool_and_selection_state() {
        assert!(super::viewport_hint_text(super::ViewportTool::Select, None)
            .contains("Drag empty space to pan"));
        assert!(
            super::viewport_hint_text(super::ViewportTool::AddCircle, None)
                .contains("Click to place a circle")
        );
        assert!(
            super::viewport_hint_text(super::ViewportTool::AddBox, Some(3)).contains("object 3")
        );
    }
}
