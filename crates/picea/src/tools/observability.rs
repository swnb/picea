use std::{
    collections::BTreeMap,
    fs,
    hash::{Hash, Hasher},
    path::Path,
    time::Instant,
};

use serde::{Deserialize, Serialize};

use crate::{
    element::ID,
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
    scene::{Scene, SceneTickObserver, SceneTickPhase},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LabArtifacts {
    pub trace_events: Vec<TraceEvent>,
    pub final_snapshot: FinalSnapshot,
    pub debug_render: DebugRenderFrame,
    pub perf: PerfSummary,
}

impl LabArtifacts {
    pub fn state_hash(&self) -> String {
        self.final_snapshot.state_hash()
    }

    pub fn diff(&self, other: &Self) -> ArtifactDiff {
        let left_hash = self.state_hash();
        let right_hash = other.state_hash();
        let first_divergent_event_index = self
            .trace_events
            .iter()
            .zip(&other.trace_events)
            .position(|(left, right)| {
                left.phase != right.phase
                    || left.tick != right.tick
                    || left.substep != right.substep
                    || left.event_kind != right.event_kind
                    || left.element_ids != right.element_ids
                    || left.pair_id != right.pair_id
                    || left.manifold_id != right.manifold_id
                    || left.reason != right.reason
                    || left.values != right.values
            });

        let first_divergent_event_index = first_divergent_event_index.or_else(|| {
            (self.trace_events.len() != other.trace_events.len())
                .then_some(self.trace_events.len().min(other.trace_events.len()))
        });
        let first_divergent_event = first_divergent_event_index.and_then(|index| {
            self.trace_events
                .get(index)
                .or_else(|| other.trace_events.get(index))
        });

        ArtifactDiff {
            left_run_id: self.final_snapshot.run_id.clone(),
            right_run_id: other.final_snapshot.run_id.clone(),
            left_state_hash: left_hash.clone(),
            right_state_hash: right_hash.clone(),
            same_state: left_hash == right_hash,
            first_divergent_event_index,
            first_divergent_tick: first_divergent_event.map(|event| event.tick),
            first_divergent_substep: first_divergent_event.map(|event| event.substep.unwrap_or(0)),
            element_count_delta: other.final_snapshot.elements.len() as isize
                - self.final_snapshot.elements.len() as isize,
            contact_count_delta: other.final_snapshot.contacts.len() as isize
                - self.final_snapshot.contacts.len() as isize,
        }
    }

    pub fn to_trace_jsonl(&self) -> Result<String, serde_json::Error> {
        self.trace_events
            .iter()
            .map(|event| serde_json::to_string(&sanitized_trace_event(event)))
            .collect::<Result<Vec<_>, _>>()
            .map(|lines| lines.join("\n"))
    }

    pub fn to_perfetto_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&PerfettoTrace {
            trace_events: self.perfetto_events(),
        })
    }

    pub fn write_to_dir(&self, dir: impl AsRef<Path>) -> std::io::Result<()> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        fs::write(
            dir.join("trace.jsonl"),
            self.to_trace_jsonl().map_err(json_to_io_error)?,
        )?;
        write_json(dir.join("final_snapshot.json"), &self.final_snapshot)?;
        write_json(dir.join("debug_render.json"), &self.debug_render)?;
        write_json(dir.join("perf.json"), &self.perf)?;
        fs::write(
            dir.join("trace.perfetto.json"),
            self.to_perfetto_json().map_err(json_to_io_error)?,
        )?;
        Ok(())
    }

    pub fn read_from_dir(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = dir.as_ref();
        let trace_events = fs::read_to_string(dir.join("trace.jsonl"))?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str::<TraceEvent>(line).map_err(json_to_io_error))
            .collect::<Result<Vec<_>, _>>()?;
        let final_snapshot = read_json(dir.join("final_snapshot.json"))?;
        let debug_render = read_json(dir.join("debug_render.json"))?;
        let perf = read_json(dir.join("perf.json"))?;

        Ok(Self {
            trace_events,
            final_snapshot,
            debug_render,
            perf,
        })
    }

    fn perfetto_events(&self) -> Vec<PerfettoEvent> {
        let mut events = Vec::new();
        events.push(PerfettoEvent::metadata("process_name", "Picea Lab"));

        for (index, event) in self.trace_events.iter().enumerate() {
            let timestamp_us = (index as u64) * 1000;
            events.push(PerfettoEvent::instant(
                event.phase,
                event.event_kind,
                timestamp_us,
                event,
            ));
        }

        for counter in &self.perf.counters {
            events.push(PerfettoEvent::counter(counter.name.clone(), counter.value));
        }

        events
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactDiff {
    pub left_run_id: String,
    pub right_run_id: String,
    pub left_state_hash: String,
    pub right_state_hash: String,
    pub same_state: bool,
    pub first_divergent_event_index: Option<usize>,
    pub first_divergent_tick: Option<u128>,
    pub first_divergent_substep: Option<u32>,
    pub element_count_delta: isize,
    pub contact_count_delta: isize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub run_id: String,
    pub tick: u128,
    pub frame: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<u64>,
    pub phase: LabPhase,
    pub substep: Option<u32>,
    pub event_kind: LabEventKind,
    pub element_ids: Vec<ID>,
    pub pair_id: Option<[ID; 2]>,
    pub manifold_id: Option<String>,
    pub reason: Option<String>,
    pub values: BTreeMap<String, FloatNum>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfettoTrace {
    pub trace_events: Vec<PerfettoEvent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerfettoEvent {
    pub name: String,
    pub cat: String,
    pub ph: String,
    pub pid: u32,
    pub tid: u32,
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dur: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<BTreeMap<String, serde_json::Value>>,
}

impl PerfettoEvent {
    fn metadata(name: &str, process_name: &str) -> Self {
        let mut args = BTreeMap::new();
        args.insert(
            "name".to_owned(),
            serde_json::Value::String(process_name.to_owned()),
        );
        Self {
            name: name.to_owned(),
            cat: "metadata".to_owned(),
            ph: "M".to_owned(),
            pid: 1,
            tid: 1,
            ts: 0,
            dur: None,
            args: Some(args),
        }
    }

    fn instant(
        phase: LabPhase,
        event_kind: LabEventKind,
        timestamp_us: u64,
        event: &TraceEvent,
    ) -> Self {
        let mut args = BTreeMap::new();
        args.insert(
            "run_id".to_owned(),
            serde_json::Value::String(event.run_id.clone()),
        );
        args.insert(
            "tick".to_owned(),
            serde_json::Value::from(event.tick as u64),
        );
        args.insert(
            "elements".to_owned(),
            serde_json::Value::from(event.element_ids.len() as u64),
        );
        for (name, value) in &event.values {
            args.insert(name.clone(), serde_json::Value::from(finite_float(*value)));
        }

        let timestamp_us = event.timestamp_us.unwrap_or(timestamp_us);
        let duration_us = event.duration_us;

        Self {
            name: format!("{:?}", event_kind),
            cat: format!("{:?}", phase),
            ph: if duration_us.is_some() {
                "X".to_owned()
            } else {
                "I".to_owned()
            },
            pid: 1,
            tid: 1,
            ts: timestamp_us,
            dur: duration_us,
            args: Some(args),
        }
    }

    fn counter(name: String, value: FloatNum) -> Self {
        let mut args = BTreeMap::new();
        args.insert(name.clone(), serde_json::Value::from(finite_float(value)));
        Self {
            name,
            cat: "picea_counters".to_owned(),
            ph: "C".to_owned(),
            pid: 1,
            tid: 1,
            ts: 0,
            dur: None,
            args: Some(args),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabPhase {
    SceneTick,
    IntegrateVelocity,
    CollisionDetect,
    WarmStart,
    ContactRefresh,
    PreSolve,
    VelocitySolve,
    PositionIntegrate,
    PositionFix,
    SleepCheck,
    TransformSync,
    PostSolve,
    DebugRender,
    Perf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabEventKind {
    PhaseBegin,
    SnapshotCaptured,
    ManifoldLifecycle,
    ContactCaptured,
    DebugRenderCaptured,
    PerfCountersCaptured,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FinalSnapshot {
    pub run_id: String,
    pub tick: u128,
    pub frame: u128,
    pub scene_state: SceneStateSnapshot,
    pub elements: Vec<ElementSnapshot>,
    pub active_pairs: Vec<ActivePairSnapshot>,
    pub contacts: Vec<ContactSnapshot>,
    pub manifolds: Vec<ManifoldSnapshot>,
}

impl FinalSnapshot {
    pub fn state_hash(&self) -> String {
        let mut hasher = StableHasher::default();
        self.tick.hash(&mut hasher);
        quantize_hash_float(self.scene_state.total_duration).hash(&mut hasher);

        for element in &self.elements {
            element.id.hash(&mut hasher);
            quantize_hash_float(element.center.x).hash(&mut hasher);
            quantize_hash_float(element.center.y).hash(&mut hasher);
            quantize_hash_float(element.velocity.x).hash(&mut hasher);
            quantize_hash_float(element.velocity.y).hash(&mut hasher);
            quantize_hash_float(element.angle_velocity).hash(&mut hasher);
            element.is_fixed.hash(&mut hasher);
            element.is_sleeping.hash(&mut hasher);
        }

        for contact in &self.contacts {
            contact.contact_id.hash(&mut hasher);
            quantize_hash_float(contact.depth).hash(&mut hasher);
            quantize_hash_float(contact.cached_normal_lambda).hash(&mut hasher);
            quantize_hash_float(contact.cached_friction_lambda).hash(&mut hasher);
        }

        format!("{:016x}", hasher.finish())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneStateSnapshot {
    pub element_count: usize,
    pub total_duration: FloatNum,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElementSnapshot {
    pub id: ID,
    pub center: LabPoint,
    pub velocity: LabVector,
    pub angle_velocity: FloatNum,
    pub is_fixed: bool,
    pub is_sleeping: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActivePairSnapshot {
    pub element_ids: [ID; 2],
    pub contact_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManifoldSnapshot {
    pub element_ids: [ID; 2],
    pub is_active: bool,
    pub contact_point_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactSnapshot {
    pub element_ids: [ID; 2],
    pub contact_index: usize,
    pub contact_id: String,
    pub point: LabPoint,
    pub point_a: LabPoint,
    pub point_b: LabPoint,
    pub normal_toward_a: LabVector,
    pub depth: FloatNum,
    pub cached_normal_lambda: FloatNum,
    pub cached_friction_lambda: FloatNum,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugRenderFrame {
    pub run_id: String,
    pub tick: u128,
    pub frame: u128,
    pub world_bounds: Option<WorldBounds>,
    pub shapes: Vec<DebugShape>,
    pub contacts: Vec<DebugContact>,
    pub manifold_labels: Vec<DebugLabel>,
    pub overlay_text: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldBounds {
    pub min: LabPoint,
    pub max: LabPoint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugShape {
    pub element_id: ID,
    pub center: LabPoint,
    pub edges: Vec<DebugEdge>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugEdge {
    Line {
        start: LabPoint,
        end: LabPoint,
    },
    Arc {
        start: LabPoint,
        support: LabPoint,
        end: LabPoint,
    },
    Circle {
        center: LabPoint,
        radius: FloatNum,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugContact {
    pub element_ids: [ID; 2],
    pub contact_id: String,
    pub point: LabPoint,
    pub normal_toward_a: LabVector,
    pub depth: FloatNum,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugLabel {
    pub element_ids: [ID; 2],
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerfSummary {
    pub run_id: String,
    pub tick: u128,
    pub frame: u128,
    pub counters: Vec<PerfCounter>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerfCounter {
    pub name: String,
    pub value: FloatNum,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LabPoint {
    pub x: FloatNum,
    pub y: FloatNum,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LabVector {
    pub x: FloatNum,
    pub y: FloatNum,
}

pub fn capture_scene_artifacts<T: Clone + Default>(
    run_id: impl Into<String>,
    scene: &Scene<T>,
) -> LabArtifacts {
    let run_id = run_id.into();
    let final_snapshot = capture_final_snapshot(&run_id, scene);
    let debug_render = capture_debug_render(&run_id, scene, &final_snapshot.contacts);
    let perf = capture_perf_summary(&run_id, scene, &final_snapshot);
    let trace_events = capture_trace_events(scene, &run_id, &final_snapshot, &debug_render, &perf);

    LabArtifacts {
        trace_events,
        final_snapshot,
        debug_render,
        perf,
    }
}

pub fn write_scene_artifacts<T: Clone + Default>(
    run_id: impl Into<String>,
    scene: &Scene<T>,
    dir: impl AsRef<Path>,
) -> std::io::Result<LabArtifacts> {
    let artifacts = capture_scene_artifacts(run_id, scene);
    artifacts.write_to_dir(dir)?;
    Ok(artifacts)
}

pub fn run_observed_tick<T: Clone + Default>(
    run_id: impl Into<String>,
    scene: &mut Scene<T>,
    delta_time: FloatNum,
) -> LabArtifacts {
    let run_id = run_id.into();
    let mut observer = LabPhaseRecorder::new(run_id.clone());
    scene.tick_observed(delta_time, &mut observer);
    let mut artifacts = capture_scene_artifacts(run_id, scene);
    observer.events.append(&mut artifacts.trace_events);
    artifacts.trace_events = observer.events;
    artifacts
}

pub fn run_observed_ticks<T, I>(
    run_id: impl Into<String>,
    scene: &mut Scene<T>,
    deltas: I,
) -> LabArtifacts
where
    T: Clone + Default,
    I: IntoIterator<Item = FloatNum>,
{
    let run_id = run_id.into();
    let mut all_events = Vec::new();
    let mut final_artifacts = None;

    for delta_time in deltas {
        let mut artifacts = run_observed_tick(run_id.clone(), scene, delta_time);
        all_events.append(&mut artifacts.trace_events);
        final_artifacts = Some(artifacts);
    }

    let mut artifacts = final_artifacts.unwrap_or_else(|| capture_scene_artifacts(run_id, scene));
    if !all_events.is_empty() {
        artifacts.trace_events = all_events;
    }
    artifacts
}

fn capture_final_snapshot<T: Clone + Default>(run_id: &str, scene: &Scene<T>) -> FinalSnapshot {
    let elements = scene
        .elements_iter()
        .map(|element| ElementSnapshot {
            id: element.id(),
            center: lab_point(element.center_point()),
            velocity: lab_vector(*element.meta().velocity()),
            angle_velocity: element.meta().angle_velocity(),
            is_fixed: element.meta().is_fixed(),
            is_sleeping: element.meta().is_sleeping(),
        })
        .collect::<Vec<_>>();

    let mut active_pairs = Vec::new();
    let mut contacts = Vec::new();
    let mut manifolds = Vec::new();

    for (pair, constraint) in scene
        .contact_constraints_manifold
        .active_constraints_with_keys()
    {
        let element_ids = [pair.0, pair.1];
        let contact_count = constraint.contact_point_pair_len();
        active_pairs.push(ActivePairSnapshot {
            element_ids,
            contact_count,
        });
        manifolds.push(ManifoldSnapshot {
            element_ids,
            is_active: true,
            contact_point_count: contact_count,
        });

        for (contact_index, contact_info) in
            constraint.contact_pair_constraint_infos_iter().enumerate()
        {
            let point = *contact_info.point();
            let point_a = *contact_info.point_a();
            let point_b = *contact_info.point_b();
            let normal_toward_a = contact_info.normal_toward_a();
            contacts.push(ContactSnapshot {
                element_ids,
                contact_index,
                contact_id: contact_digest(scene, element_ids, point_a, point_b, normal_toward_a),
                point: lab_point(point),
                point_a: lab_point(point_a),
                point_b: lab_point(point_b),
                normal_toward_a: lab_vector(normal_toward_a),
                depth: finite_float(contact_info.depth()),
                cached_normal_lambda: finite_float(contact_info.total_lambda()),
                cached_friction_lambda: finite_float(contact_info.total_friction_lambda()),
            });
        }
    }

    FinalSnapshot {
        run_id: run_id.to_owned(),
        tick: scene.frame_count(),
        frame: scene.frame_count(),
        scene_state: SceneStateSnapshot {
            element_count: scene.element_size(),
            total_duration: scene.total_duration(),
        },
        elements,
        active_pairs,
        contacts,
        manifolds,
    }
}

fn capture_debug_render<T: Clone + Default>(
    run_id: &str,
    scene: &Scene<T>,
    contacts: &[ContactSnapshot],
) -> DebugRenderFrame {
    let shapes = scene
        .elements_iter()
        .map(|element| DebugShape {
            element_id: element.id(),
            center: lab_point(element.center_point()),
            edges: element.shape().edge_iter().map(debug_edge).collect(),
        })
        .collect::<Vec<_>>();

    let debug_contacts = contacts
        .iter()
        .map(|contact| DebugContact {
            element_ids: contact.element_ids,
            contact_id: contact.contact_id.clone(),
            point: contact.point,
            normal_toward_a: contact.normal_toward_a,
            depth: contact.depth,
        })
        .collect::<Vec<_>>();

    let manifold_labels = contacts
        .iter()
        .map(|contact| DebugLabel {
            element_ids: contact.element_ids,
            text: format!(
                "{}-{} contact#{}",
                contact.element_ids[0], contact.element_ids[1], contact.contact_index
            ),
        })
        .collect();

    DebugRenderFrame {
        run_id: run_id.to_owned(),
        tick: scene.frame_count(),
        frame: scene.frame_count(),
        world_bounds: compute_world_bounds(&shapes),
        shapes,
        contacts: debug_contacts,
        manifold_labels,
        overlay_text: vec![format!(
            "tick={} elements={} active_pairs={}",
            scene.frame_count(),
            scene.element_size(),
            scene
                .contact_constraints_manifold
                .active_constraints_with_keys()
                .count()
        )],
    }
}

fn capture_perf_summary<T: Clone + Default>(
    run_id: &str,
    scene: &Scene<T>,
    snapshot: &FinalSnapshot,
) -> PerfSummary {
    PerfSummary {
        run_id: run_id.to_owned(),
        tick: scene.frame_count(),
        frame: scene.frame_count(),
        counters: vec![
            PerfCounter {
                name: "element_count".to_owned(),
                value: finite_float(scene.element_size() as FloatNum),
            },
            PerfCounter {
                name: "active_manifold_count".to_owned(),
                value: finite_float(snapshot.active_pairs.len() as FloatNum),
            },
            PerfCounter {
                name: "contact_count".to_owned(),
                value: finite_float(snapshot.contacts.len() as FloatNum),
            },
        ],
    }
}

fn capture_trace_events<T: Clone + Default>(
    scene: &Scene<T>,
    run_id: &str,
    snapshot: &FinalSnapshot,
    debug_render: &DebugRenderFrame,
    perf: &PerfSummary,
) -> Vec<TraceEvent> {
    let mut events = Vec::new();
    events.push(TraceEvent {
        run_id: run_id.to_owned(),
        tick: scene.frame_count(),
        frame: scene.frame_count(),
        timestamp_us: None,
        duration_us: None,
        phase: LabPhase::SceneTick,
        substep: None,
        event_kind: LabEventKind::SnapshotCaptured,
        element_ids: snapshot.elements.iter().map(|element| element.id).collect(),
        pair_id: None,
        manifold_id: None,
        reason: Some("headless_capture".to_owned()),
        values: counter_values(&[
            (
                "element_count",
                snapshot.scene_state.element_count as FloatNum,
            ),
            ("active_pair_count", snapshot.active_pairs.len() as FloatNum),
            ("contact_count", snapshot.contacts.len() as FloatNum),
        ]),
    });

    for contact in &snapshot.contacts {
        events.push(TraceEvent {
            run_id: run_id.to_owned(),
            tick: scene.frame_count(),
            frame: scene.frame_count(),
            timestamp_us: None,
            duration_us: None,
            phase: LabPhase::CollisionDetect,
            substep: None,
            event_kind: LabEventKind::ContactCaptured,
            element_ids: contact.element_ids.to_vec(),
            pair_id: Some(contact.element_ids),
            manifold_id: Some(format!(
                "{}-{}",
                contact.element_ids[0], contact.element_ids[1]
            )),
            reason: Some("active_contact".to_owned()),
            values: counter_values(&[
                ("depth", contact.depth),
                ("cached_normal_lambda", contact.cached_normal_lambda),
                ("cached_friction_lambda", contact.cached_friction_lambda),
            ]),
        });
    }

    events.push(TraceEvent {
        run_id: run_id.to_owned(),
        tick: scene.frame_count(),
        frame: scene.frame_count(),
        timestamp_us: None,
        duration_us: None,
        phase: LabPhase::DebugRender,
        substep: None,
        event_kind: LabEventKind::DebugRenderCaptured,
        element_ids: Vec::new(),
        pair_id: None,
        manifold_id: None,
        reason: Some("render_facts_captured".to_owned()),
        values: counter_values(&[
            ("shape_count", debug_render.shapes.len() as FloatNum),
            ("contact_count", debug_render.contacts.len() as FloatNum),
        ]),
    });

    events.push(TraceEvent {
        run_id: run_id.to_owned(),
        tick: scene.frame_count(),
        frame: scene.frame_count(),
        timestamp_us: None,
        duration_us: None,
        phase: LabPhase::Perf,
        substep: None,
        event_kind: LabEventKind::PerfCountersCaptured,
        element_ids: Vec::new(),
        pair_id: None,
        manifold_id: None,
        reason: Some("perf_counters_captured".to_owned()),
        values: perf
            .counters
            .iter()
            .map(|counter| (counter.name.clone(), counter.value))
            .collect(),
    });

    events
}

struct LabPhaseRecorder {
    run_id: String,
    events: Vec<TraceEvent>,
    started_at: Instant,
    last_phase_event_index: Option<usize>,
}

impl LabPhaseRecorder {
    fn new(run_id: String) -> Self {
        Self {
            run_id,
            events: Vec::new(),
            started_at: Instant::now(),
            last_phase_event_index: None,
        }
    }
}

impl SceneTickObserver for LabPhaseRecorder {
    fn on_phase(&mut self, tick: u128, substep: u32, phase: SceneTickPhase) {
        let timestamp_us = self.started_at.elapsed().as_micros() as u64;
        if let Some(previous_index) = self.last_phase_event_index {
            let previous_timestamp = self.events[previous_index].timestamp_us.unwrap_or(0);
            self.events[previous_index].duration_us =
                Some(timestamp_us.saturating_sub(previous_timestamp));
        }

        self.events.push(TraceEvent {
            run_id: self.run_id.clone(),
            tick,
            frame: tick,
            timestamp_us: Some(timestamp_us),
            duration_us: None,
            phase: lab_phase_from_scene_phase(phase),
            substep: Some(substep),
            event_kind: LabEventKind::PhaseBegin,
            element_ids: Vec::new(),
            pair_id: None,
            manifold_id: None,
            reason: Some("observed_tick".to_owned()),
            values: BTreeMap::new(),
        });
        self.last_phase_event_index = Some(self.events.len() - 1);
    }
}

fn lab_phase_from_scene_phase(phase: SceneTickPhase) -> LabPhase {
    match phase {
        SceneTickPhase::StepBegin => LabPhase::SceneTick,
        SceneTickPhase::IntegrateVelocity => LabPhase::IntegrateVelocity,
        SceneTickPhase::CollisionDetect => LabPhase::CollisionDetect,
        SceneTickPhase::WarmStart => LabPhase::WarmStart,
        SceneTickPhase::ContactRefresh => LabPhase::ContactRefresh,
        SceneTickPhase::PreSolve => LabPhase::PreSolve,
        SceneTickPhase::VelocitySolve => LabPhase::VelocitySolve,
        SceneTickPhase::PositionIntegrate => LabPhase::PositionIntegrate,
        SceneTickPhase::PositionFix => LabPhase::PositionFix,
        SceneTickPhase::SleepCheck => LabPhase::SleepCheck,
        SceneTickPhase::TransformSync => LabPhase::TransformSync,
        SceneTickPhase::PostSolve => LabPhase::PostSolve,
    }
}

fn counter_values(values: &[(&str, FloatNum)]) -> BTreeMap<String, FloatNum> {
    values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), finite_float(*value)))
        .collect()
}

fn sanitized_trace_event(event: &TraceEvent) -> TraceEvent {
    let mut sanitized = event.clone();
    sanitized.values = sanitized
        .values
        .into_iter()
        .map(|(name, value)| (name, finite_float(value)))
        .collect();
    sanitized
}

fn debug_edge(edge: Edge<'_>) -> DebugEdge {
    match edge {
        Edge::Line {
            start_point,
            end_point,
        } => DebugEdge::Line {
            start: lab_point(*start_point),
            end: lab_point(*end_point),
        },
        Edge::Arc {
            start_point,
            support_point,
            end_point,
        } => DebugEdge::Arc {
            start: lab_point(*start_point),
            support: lab_point(*support_point),
            end: lab_point(*end_point),
        },
        Edge::Circle {
            center_point,
            radius,
        } => DebugEdge::Circle {
            center: lab_point(center_point),
            radius,
        },
    }
}

fn compute_world_bounds(shapes: &[DebugShape]) -> Option<WorldBounds> {
    let mut points = shapes.iter().flat_map(debug_shape_points);
    let first = points.next()?;
    let mut min_x = first.x;
    let mut max_x = first.x;
    let mut min_y = first.y;
    let mut max_y = first.y;

    for point in points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }

    Some(WorldBounds {
        min: LabPoint { x: min_x, y: min_y },
        max: LabPoint { x: max_x, y: max_y },
    })
}

fn debug_shape_points(shape: &DebugShape) -> Vec<LabPoint> {
    let mut points = vec![shape.center];
    for edge in &shape.edges {
        match edge {
            DebugEdge::Line { start, end } => {
                points.push(*start);
                points.push(*end);
            }
            DebugEdge::Arc {
                start,
                support,
                end,
            } => {
                points.push(*start);
                points.push(*support);
                points.push(*end);
            }
            DebugEdge::Circle { center, radius } => {
                points.push(LabPoint {
                    x: center.x - *radius,
                    y: center.y - *radius,
                });
                points.push(LabPoint {
                    x: center.x + *radius,
                    y: center.y + *radius,
                });
            }
        }
    }
    points
}

fn lab_point(point: Point) -> LabPoint {
    LabPoint {
        x: finite_float(point.x()),
        y: finite_float(point.y()),
    }
}

fn lab_vector(vector: Vector) -> LabVector {
    LabVector {
        x: finite_float(vector.x()),
        y: finite_float(vector.y()),
    }
}

fn finite_float(value: FloatNum) -> FloatNum {
    if value.is_finite() {
        value
    } else {
        0.
    }
}

fn contact_digest<T: Clone + Default>(
    scene: &Scene<T>,
    element_ids: [ID; 2],
    point_a: Point,
    point_b: Point,
    normal_toward_a: Vector,
) -> String {
    let anchor_a = scene
        .get_element(element_ids[0])
        .map(|element| Vector::from((element.center_point(), point_a)))
        .unwrap_or_else(|| point_a.to_vector());
    let anchor_b = scene
        .get_element(element_ids[1])
        .map(|element| Vector::from((element.center_point(), point_b)))
        .unwrap_or_else(|| point_b.to_vector());
    let normal = normal_toward_a.normalize();

    format!(
        "{}-{}:a={},{}:b={},{}:n={},{}",
        element_ids[0],
        element_ids[1],
        quantize_digest_component(anchor_a.x()),
        quantize_digest_component(anchor_a.y()),
        quantize_digest_component(anchor_b.x()),
        quantize_digest_component(anchor_b.y()),
        quantize_digest_component(normal.x()),
        quantize_digest_component(normal.y())
    )
}

fn quantize_digest_component(value: FloatNum) -> i32 {
    if !value.is_finite() {
        return 0;
    }

    let quantized = (value * 1000.).round();
    if quantized > i32::MAX as FloatNum {
        i32::MAX
    } else if quantized < i32::MIN as FloatNum {
        i32::MIN
    } else {
        quantized as i32
    }
}

fn quantize_hash_float(value: FloatNum) -> i64 {
    (finite_float(value) * 1000.).round() as i64
}

#[derive(Default)]
struct StableHasher(u64);

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }
}

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(json_to_io_error)?;
    fs::write(path, json)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> std::io::Result<T> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(json_to_io_error)
}

fn json_to_io_error(error: serde_json::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::{capture_scene_artifacts, LabEventKind, LabPhase};
    use std::fs;

    use crate::{
        element::ElementBuilder, math::FloatNum, meta::MetaBuilder, scene::Scene, shape::Circle,
    };

    const STEP_DT: FloatNum = 1. / 60.;

    fn unticked_contact_scene() -> (Scene<()>, u32, u32) {
        let mut scene = Scene::width_capacity(2);
        scene.set_gravity(|_| (0., 0.).into());
        let element_a_id = scene.push_element(ElementBuilder::new(
            Circle::new((0., 0.), 1.),
            MetaBuilder::new().mass(1.).is_fixed(true),
            (),
        ));
        let element_b_id = scene.push_element(ElementBuilder::new(
            Circle::new((1.5, 0.), 1.),
            MetaBuilder::new().mass(1.).is_fixed(true),
            (),
        ));
        (scene, element_a_id, element_b_id)
    }

    fn contact_scene() -> (Scene<()>, u32, u32) {
        let (mut scene, element_a_id, element_b_id) = unticked_contact_scene();
        scene.tick(STEP_DT);
        (scene, element_a_id, element_b_id)
    }

    fn unique_artifact_dir(name: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("picea-lab-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn headless_capture_records_minimal_lab_artifacts_from_scene() {
        let (scene, element_a_id, element_b_id) = contact_scene();

        let artifacts = capture_scene_artifacts("run-l1", &scene);

        assert_eq!(artifacts.final_snapshot.run_id, "run-l1");
        assert_eq!(artifacts.final_snapshot.tick, scene.frame_count());
        assert_eq!(artifacts.final_snapshot.frame, scene.frame_count());
        assert_eq!(artifacts.final_snapshot.scene_state.element_count, 2);
        assert_eq!(artifacts.final_snapshot.active_pairs.len(), 1);
        assert_eq!(
            artifacts.final_snapshot.active_pairs[0].element_ids,
            [element_a_id, element_b_id]
        );
        assert!(
            !artifacts.final_snapshot.contacts[0].contact_id.is_empty(),
            "contacts need a public stable-ish digest for replay/diff"
        );

        assert_eq!(artifacts.debug_render.run_id, "run-l1");
        assert_eq!(artifacts.debug_render.shapes.len(), 2);
        assert!(artifacts.debug_render.overlay_text[0].contains("active_pairs=1"));
        assert!(
            !artifacts.debug_render.contacts.is_empty(),
            "contact scenario should produce renderable contact facts"
        );
        assert_eq!(
            artifacts.debug_render.contacts[0].contact_id,
            artifacts.final_snapshot.contacts[0].contact_id
        );

        let trace_order = artifacts
            .trace_events
            .iter()
            .map(|event| (event.phase, event.event_kind))
            .collect::<Vec<_>>();
        assert_eq!(
            trace_order.first(),
            Some(&(LabPhase::SceneTick, LabEventKind::SnapshotCaptured))
        );
        assert_eq!(
            trace_order.last(),
            Some(&(LabPhase::Perf, LabEventKind::PerfCountersCaptured))
        );
        assert!(artifacts
            .perf
            .counters
            .iter()
            .any(|counter| { counter.name == "active_manifold_count" && counter.value >= 1. }));

        let trace_jsonl = artifacts
            .to_trace_jsonl()
            .expect("trace events serialize to jsonl");
        assert_eq!(trace_jsonl.lines().count(), artifacts.trace_events.len());
        for line in trace_jsonl.lines() {
            let value: serde_json::Value =
                serde_json::from_str(line).expect("each trace jsonl line is valid json");
            assert_eq!(value["run_id"], "run-l1");
        }

        serde_json::to_value(&artifacts.final_snapshot).expect("snapshot serializes");
        serde_json::to_value(&artifacts.debug_render).expect("debug render serializes");
        serde_json::to_value(&artifacts.perf).expect("perf summary serializes");
    }

    #[test]
    fn headless_capture_does_not_mutate_scene_state_or_public_queries() {
        let (scene, _element_a_id, _element_b_id) = contact_scene();
        let frame_count = scene.frame_count();
        let total_duration = scene.total_duration();
        let position_fix_map = scene.get_position_fix_map();

        let artifacts = capture_scene_artifacts("run-noop", &scene);

        assert_eq!(scene.frame_count(), frame_count);
        assert_eq!(scene.total_duration(), total_duration);
        assert_eq!(scene.get_position_fix_map(), position_fix_map);
        assert_eq!(artifacts.final_snapshot.tick, frame_count);
    }

    #[test]
    fn trace_jsonl_sanitizes_non_finite_counter_values() {
        let (scene, _element_a_id, _element_b_id) = contact_scene();
        let mut artifacts = capture_scene_artifacts("run-non-finite", &scene);
        artifacts.trace_events[0]
            .values
            .insert("bad".to_owned(), FloatNum::NAN);
        artifacts.trace_events[0]
            .values
            .insert("huge".to_owned(), FloatNum::INFINITY);

        let trace_jsonl = artifacts
            .to_trace_jsonl()
            .expect("non-finite values are sanitized before jsonl export");
        let first_line: serde_json::Value =
            serde_json::from_str(trace_jsonl.lines().next().expect("first event exists"))
                .expect("sanitized line is valid json");

        assert_eq!(first_line["values"]["bad"], 0.);
        assert_eq!(first_line["values"]["huge"], 0.);
    }

    #[test]
    fn artifact_bundle_writes_and_reads_standard_files() {
        let (scene, _element_a_id, _element_b_id) = contact_scene();
        let dir = unique_artifact_dir("write-read");

        let written = super::write_scene_artifacts("run-files", &scene, &dir)
            .expect("artifacts write to dir");
        let loaded = super::LabArtifacts::read_from_dir(&dir).expect("artifacts load from dir");

        assert_eq!(written.state_hash(), loaded.state_hash());
        assert!(dir.join("trace.jsonl").is_file());
        assert!(dir.join("final_snapshot.json").is_file());
        assert!(dir.join("debug_render.json").is_file());
        assert!(dir.join("perf.json").is_file());
        assert!(dir.join("trace.perfetto.json").is_file());

        fs::remove_dir_all(&dir).expect("cleanup artifact dir");
    }

    #[test]
    fn state_hash_and_diff_are_stable_across_run_ids_and_detect_state_changes() {
        let (scene, _element_a_id, _element_b_id) = contact_scene();
        let left = capture_scene_artifacts("left", &scene);
        let right = capture_scene_artifacts("right", &scene);
        assert_eq!(left.state_hash(), right.state_hash());
        assert!(left.diff(&right).same_state);

        let mut changed_scene = scene;
        changed_scene.push_element(ElementBuilder::new(
            Circle::new((8., 0.), 1.),
            MetaBuilder::new().mass(1.),
            (),
        ));
        let changed = capture_scene_artifacts("changed", &changed_scene);
        let diff = left.diff(&changed);

        assert!(!diff.same_state);
        assert_eq!(diff.element_count_delta, 1);
        assert!(diff.first_divergent_event_index.is_some());
    }

    #[test]
    fn perfetto_export_contains_trace_events_and_counters() {
        let (scene, _element_a_id, _element_b_id) = contact_scene();
        let artifacts = capture_scene_artifacts("run-perfetto", &scene);

        let perfetto_json = artifacts
            .to_perfetto_json()
            .expect("perfetto json serializes");
        let value: serde_json::Value =
            serde_json::from_str(&perfetto_json).expect("perfetto json parses");
        let trace_events = value["traceEvents"]
            .as_array()
            .expect("traceEvents is an array");

        assert!(trace_events.iter().any(|event| event["ph"] == "M"));
        assert!(trace_events.iter().any(|event| event["ph"] == "I"));
        assert!(trace_events.iter().any(|event| event["ph"] == "C"));
        assert!(trace_events
            .iter()
            .any(|event| event["cat"] == "picea_counters"));
    }

    #[test]
    fn observed_tick_records_phase_order_without_changing_physics_result() {
        let (mut observed_scene, _observed_a, _observed_b) = unticked_contact_scene();
        let (mut regular_scene, _regular_a, _regular_b) = unticked_contact_scene();

        let artifacts = super::run_observed_tick("run-observed", &mut observed_scene, STEP_DT);
        regular_scene.tick(STEP_DT);

        assert_eq!(observed_scene.frame_count(), regular_scene.frame_count());
        assert_eq!(
            observed_scene.get_position_fix_map(),
            regular_scene.get_position_fix_map()
        );
        assert_eq!(artifacts.final_snapshot.tick, observed_scene.frame_count());

        let phases = artifacts
            .trace_events
            .iter()
            .filter(|event| event.event_kind == LabEventKind::PhaseBegin)
            .map(|event| event.phase)
            .collect::<Vec<_>>();

        assert_eq!(
            phases,
            vec![
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
            ]
        );

        let phase_events = artifacts
            .trace_events
            .iter()
            .filter(|event| event.event_kind == LabEventKind::PhaseBegin)
            .collect::<Vec<_>>();
        assert!(phase_events.iter().any(|event| event.duration_us.is_some()));

        let perfetto_json = artifacts
            .to_perfetto_json()
            .expect("observed tick perfetto export serializes");
        let value: serde_json::Value =
            serde_json::from_str(&perfetto_json).expect("perfetto json parses");
        assert!(value["traceEvents"]
            .as_array()
            .expect("traceEvents is an array")
            .iter()
            .any(|event| event["ph"] == "X" && event.get("dur").is_some()));
    }

    #[test]
    fn observed_ticks_replay_accumulates_events_and_final_snapshot() {
        let (mut scene, _element_a_id, _element_b_id) = unticked_contact_scene();

        let artifacts =
            super::run_observed_ticks("run-three-steps", &mut scene, [STEP_DT, STEP_DT, STEP_DT]);

        assert_eq!(artifacts.final_snapshot.tick, 3);
        assert_eq!(artifacts.final_snapshot.frame, 3);
        assert_eq!(
            artifacts.trace_events.first().map(|event| event.tick),
            Some(1)
        );
        assert_eq!(
            artifacts.trace_events.last().map(|event| event.tick),
            Some(3)
        );
        assert_eq!(
            artifacts
                .trace_events
                .iter()
                .filter(|event| {
                    event.event_kind == LabEventKind::PhaseBegin
                        && event.phase == LabPhase::SceneTick
                })
                .count(),
            3
        );
    }

    #[test]
    fn artifact_diff_treats_logical_tick_and_substep_as_event_identity() {
        let (mut scene, _element_a_id, _element_b_id) = unticked_contact_scene();
        let left = super::run_observed_ticks("left", &mut scene, [STEP_DT, STEP_DT, STEP_DT]);
        let mut shifted = left.clone();
        shifted.final_snapshot.run_id = "shifted".to_owned();
        shifted.trace_events[0].run_id = "shifted".to_owned();
        shifted.trace_events[0].tick += 1;

        let diff = left.diff(&shifted);

        assert!(diff.same_state);
        assert_eq!(diff.first_divergent_event_index, Some(0));
        assert_eq!(diff.first_divergent_tick, Some(1));
        assert_eq!(diff.first_divergent_substep, Some(0));
    }
}
