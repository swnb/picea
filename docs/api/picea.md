---
title: picea API 文档
---

# picea 物理引擎 API 文档

`picea` 是一个 2D 刚体物理引擎，提供场景管理、碰撞检测、约束求解、形状建模与数值工具。本章节列出所有对外公开的结构体、函数与 trait，并给出使用示例。

## 1. 快速开始

下面的示例展示了如何创建一个场景、添加矩形和圆形元素，并在主循环中推进物理模拟：

```rust
use picea::prelude::*;

fn main() {
    let mut scene = Scene::new();

    // 创建一个静态地面
    let ground = ElementBuilder::new(
        Rect::new(0.0, 300.0, 800.0, 40.0),
        MetaBuilder::new().mass(1000.0).is_fixed(true),
        (),
    );
    scene.push_element(ground);

    // 创建一个可运动的方块
    let box_meta = MetaBuilder::new()
        .mass(5.0)
        .factor_restitution(0.1)
        .velocity((2.0, 0.0));
    let block = ElementBuilder::new(
        Square::new((200.0, 120.0), 40.0),
        box_meta,
        (),
    );
    let block_id = scene.push_element(block);

    // 每帧调用 tick 推进世界时间
    for _frame in 0..600 {
        scene.tick(1.0 / 60.0);

        if let Some(element) = scene.get_element(block_id) {
            println!("高度: {}", element.center_point().y());
        }
    }
}
```

## 2. 模块导航

| 模块 | 说明 |
| --- | --- |
| `prelude` | 常用类型重导出，便于快速引入引擎核心能力 |
| `scene` | 场景生命周期管理、约束调度、回调注册等 |
| `element` | 物体结构定义、构建器、克隆与惯量计算 |
| `shape` | 几何体实现及通用变换、最近点计算工具 |
| `math` | 数学基础类型（向量、点、矩阵、常量等） |
| `meta` | 元数据与物理属性（质量、速度、变换、力） |
| `constraints` | 点约束、关节约束及软约束参数求解 |
| `collision` | 碰撞检测流程、GJK/EPA 支持点与接触对 |
| `tools` | 调试与辅助工具（拖拽、快照、碰撞视图） |
| `algo` | 引擎内部排序算法实现（对外仅接口说明） |

## 3. Prelude

`picea::prelude` 重导出以下类型，推荐在项目入口 `use picea::prelude::*;`：

- `Scene`
- 元素相关：`ElementBuilder`, `ShapeTraitUnion`, `ComputeMomentOfInertia`, `SelfClone`, `ID`
- 数学类型：`Point`, `Vector`, `Segment`, `Edge`, `FloatNum`
- 元数据：`Mass`, `Meta`, `MetaBuilder`
- 形状 trait：`CenterPoint`, `EdgeIterable`, `GeometryTransformer`, `NearestPoint`
- 约束配置：`JoinConstraintConfig`, `JoinConstraintConfigBuilder`
- 碰撞接口：`Projector`

## 4. 场景 (`scene` 模块)

### 4.1 `Scene<Data = ()>`

`Scene` 是引擎的顶级调度器，泛型 `Data` 用于存储用户自定义的全局状态（可选，需实现 `Clone + Default`）。关键公开方法如下：

| 方法 | 说明 |
| --- | --- |
| `Scene::new()` | 创建一个默认场景，容量按需增长 |
| `Scene::width_capacity(capacity)` | 预分配元素存储容量，降低重新分配成本 |
| `scene.push_element(ElementBuilder)` | 将元素放入场景并返回元素 ID |
| `scene.has_element(id)` / `remove_element(id)` | 检查或移除元素 |
| `scene.element_size()` | 返回当前元素数量 |
| `scene.tick(delta_time)` | 推进模拟，内部完成速度积分、碰撞检测、约束求解与位置修正 |
| `scene.total_duration()` | 累计经过的模拟时间 |
| `scene.register_element_position_update_callback(f)` | 注册元素位置变化回调，返回回调 ID |
| `scene.unregister_element_position_update_callback(id)` | 取消回调 |
| `scene.elements_iter()` / `elements_iter_mut()` | 迭代所有元素（不可变/可变） |
| `scene.get_element(id)` / `get_element_mut(id)` | 按 ID 查询元素 |
| `scene.frame_count()` | 返回累计帧数 |
| `scene.context_mut()` | 获取 `Context` 进行全局配置（约束参数、重力、睡眠等） |
| `scene.clear()` | 清空所有元素与约束，并重置帧计数 |
| `scene.is_element_collide(a_id, b_id, query_from_manifold)` | 查询两个元素是否碰撞，可选择是否直接使用当前接触流形 |
| `scene.set_gravity(reducer)` | 自定义重力向量（传入闭包修改默认重力） |
| `scene.create_point_constraint(...)` | 创建元素与固定点的软/硬约束，返回约束 ID |
| `scene.point_constraints()` | 遍历当前点约束 |
| `scene.get_point_constraint(id)` / `get_point_constraint_mut(id)` | 查询指定点约束 |
| `scene.remove_point_constraint(id)` | 删除点约束并解绑元素锚点 |
| `scene.create_join_constraint(...)` | 创建元素之间的关节约束 |
| `scene.join_constraints()` / `get_join_constraint(_mut)` | 查询关节约束 |
| `scene.remove_join_constraint(id)` | 移除关节并解绑元素锚点 |
| `scene.set_sleep_mode(enabled)` | 开启/关闭睡眠模式，自动沉睡低能量物体 |
| `scene.silent()` | 将所有元素的线速度与角速度置零 |
| `scene.get_position_fix_map()` | 调试当前接触求解的位移修正量 |

> 提示：`Context` 中的 `constraint_parameters` 可调节碰撞穿透修正、默认摩擦系数、最大限制力等数值，适合在初始化阶段设定。

### 4.2 `Context`

`Scene::context_mut()` 返回 `Context`，主要字段：

- `constraint_parameters` (`ConstraintParameters`)：控制解算器参数，如 `factor_position_bias`、`max_allow_permeate`、`split_position_fix` 等。
- `enable_sleep_mode`：是否启用睡眠判定。
- `max_enter_sleep_kinetic`、`max_enter_sleep_frame`：睡眠阈值。
- `enable_gravity`、`default_gravity`：全局重力开关与向量。

## 5. 元素与构建 (`element` 模块)

### 5.1 `Element<T>`

`Element` 代表场景中的刚体元素，`T` 为附加数据类型。关键方法：

- `Element::new(shape, meta, data)`：直接用形状、元数据、附加数据构造一个元素。
- `element.center_point()`：返回当前中心点。
- `element.integrate_position(delta_time)`：根据当前速度积分位置，返回 `(平移, 旋转)`。
- `element.meta()` / `meta_mut()`：访问元数据。
- `element.shape()`：借助 `Fields` 派生的 getter 获取动态形状引用。
- `element.transform(transform)`、`element.apply_transform()`：内部使用，处理增量变换。
- `element.create_bind_point(id, point)` / `get_bind_point(id)` / `remove_bind_point(id)`：管理约束绑定点。

### 5.2 `ElementBuilder<T>`

用于构建元素的便捷接口：

- `ElementBuilder::new(shape, meta, data)`：shape 必须实现 `Into<Box<dyn ShapeTraitUnion>>`，meta 可直接传入 `Meta` 或 `MetaBuilder`。
- 链式 API：`builder.shape(new_shape)`、`builder.meta(meta)`、`builder.addition_data(data)`。
- `ElementBuilder` 自动在 `Scene::push_element` 时计算形状的转动惯量并注入到 `Meta`。

### 5.3 Trait 汇总

- `ComputeMomentOfInertia`：计算给定质量下的转动惯量，所有形状需实现。
- `SelfClone`：将形状克隆为 trait 对象，用于元素复制。
- `ShapeTraitUnion`：统一抽象（`GeometryTransformer + CenterPoint + NearestPoint + EdgeIterable + ComputeMomentOfInertia + Projector + Collider + SelfClone`）。
- `ConstraintObject`（定义于 `constraints`）：约束系统使用元素的公共接口。

### 5.4 `ElementStore<T>`

`ElementStore` 是公开结构体，主要通过 `Scene` 间接使用。其提供：

- `ElementStore::with_capacity(capacity)`
- `store.iter()` / `iter_mut()`
- `store.push(element)`、`remove_element(id)`、`clear()`
- `store.get_element_by_id(id)`、`get_mut_element_by_id(id)`
- `store.detective_collision(handler)`：执行碰撞检测并回调结果（通常由 `Scene` 调用）。

## 6. 元数据 (`meta` 模块)

### 6.1 `Meta`

`Meta` 存储元素的物理属性：

- 质量 (`mass`)、转动惯量 (`moment_of_inertia`)、线速度 (`velocity`)、角速度 (`angle_velocity`)
- 摩擦系数 (`factor_friction`)、弹性系数 (`factor_restitution`)
- 状态标记：`is_fixed`、`is_transparent`、`is_ignore_gravity`、`is_sleeping`
- 累积变换：`delta_transform`、`total_transform`
- 接触统计：`contact_count`、`inactive_frame_count`

常用方法：

| 方法 | 说明 |
| --- | --- |
| `meta.mass()` / `meta.set_mass(value)` | 读取或设定质量，并自动更新倒数 |
| `meta.inv_mass()`、`meta.inv_moment_of_inertia()` | 返回倒数，求解器使用 |
| `meta.motion()` | 返回线动量向量 |
| `meta.apply_impulse(impulse, r)` | 对质心施加冲量，并根据力臂更新角速度 |
| `meta.compute_kinetic_energy()` | 计算动能 |
| `meta.compute_rough_energy()` | 返回调试用能量指标数组 |
| `meta.silent()` | 清除速度、角速度 |

### 6.2 `MetaBuilder`

`MetaBuilder::new()` 创建默认元数据，可链式调用：

- `mass(value)`、`velocity((x, y))`、`angle_velocity(value)`
- `factor_friction(value)`、`factor_restitution(value)`
- `is_fixed(bool)`、`is_transparent(bool)`、`is_ignore_gravity(bool)`

### 6.3 力系统 (`meta::force`)

- `Force::new(id, vector)`：创建具名力，可设置临时标记 `set_temporary`。
- `Force::get_vector()` / `set_vector(reducer)`：读取或更新力向量。
- `ForceGroup`：管理一组力，提供 `add_force`、`get_force(_mut)`、`remove`、`sum_force`、`iter` 等接口。

## 7. 数学工具 (`math` 模块)

- `FloatNum`：浮点类型别名（`f32`）。
- 常量函数：`pi()`、`tau()`。
- `axis::AxisDirection`：`X`、`Y` 枚举，支持取反 `!axis`。
- `point::Point<T = FloatNum>`：点坐标，支持 `Point::new(x, y)`、坐标访问、向量转换。
- `vector::Vector<T = FloatNum>` / `Vector3<T>`：包含加减乘除、点积、叉积、归一化、旋转、角度计算等常见操作。
- `segment::Segment<T>`：线段封装，可获取起终点、向量投影等。
- `edge::Edge`：多种边缘类型枚举（线段、圆弧、圆），用于形状迭代。
- `matrix`：提供基础矩阵运算（详见源码）。
- `num::is_same_sign(a, b)`：判断符号是否相同（内部使用，同样为 `pub(crate)`，不对外暴露）。

## 8. 形状 (`shape` 模块)

所有形状实现 `ShapeTraitUnion` 所需 trait，可直接用于 `ElementBuilder::new`。关键结构体及构造函数：

- `Rect::new(top_left_x, top_left_y, width, height)`
- `Square::new(center_point, edge_length)`
- `Triangle::new([Point; 3])`
- `Circle::new(center_point, radius)`
- `Line::new(start_point, end_point)`
- `ConvexPolygon::new(vertices: Vec<Point>)`
- `ConcavePolygon::new(vertices: Vec<Point>)`
- `RegularPolygon::new(center_point, edge_count, radius)`
- `ConstRegularPolygon::<N>::new(center_point, radius)`（编译期定边正多边形）
- `ConstPolygon::<N>::new(vertices: [Point; N])`

通用工具（`shape::utils`）：

- `compute_polygon_approximate_center_point`、`compute_convex_center_point`、`compute_area_of_convex`
- `check_is_polygon_clockwise`、`check_is_concave`
- 变换函数：`translate_polygon`、`rotate_polygon`、`resize_by_vector`
- 碰撞辅助：`projection_polygon_on_vector`、`check_is_segment_cross`、`compute_cross_point_between_two_segment`
- 几何拆分：`split_convex_polygon_to_triangles`、`split_clockwise_concave_polygon_to_two_convex_polygon`

## 9. 碰撞 (`collision` 模块)

- Trait
  - `Projector`：投影到向量/坐标轴，所有碰撞形状需实现。
  - `SubCollider`：具备 `Projector + CenterPoint + NearestPoint` 的子碰撞体。
  - `Collider`：整体碰撞体，可提供 `sub_colliders()` 以支持组合碰撞体。
  - `CollisionalCollection`：可排序的碰撞体集合接口，`ElementStore` 实现了此 trait。
- 流程函数
  - `detect_collision(collection, handler, skip)`：包裹粗检测、子碰撞体精确检测，并将最终接触点列表交给 `handler`。
  - `rough_collision_detection(collection, handler)`：Sweep & Prune 粗检测。
  - `prepare_accurate_collision_detection(collider_a, collider_b, handler)`：枚举子碰撞体组合。
  - `accurate_collision_detection_for_sub_collider(a, b)`：GJK + EPA 计算接触点对。
- 数据结构
  - `ContactPointPair`：包含接触点（A/B）、法线、穿透深度、平均点。
  - `MinkowskiDifferencePoint`、`MinkowskiEdge`：EPA 过程中使用的中间结构。

## 10. 约束 (`constraints` 模块)

- `JoinConstraintConfig`
  - 字段：`distance`、`damping_ratio`、`frequency`、`hard`
  - 通过 `JoinConstraintConfigBuilder` 链式设置（由 `picea_macro_tools::Builder` 生成）。

- `compute_inv_mass_effective(normal, (obj_a, obj_b), r_a, r_b)`：常规约束质量组合计算。
- `compute_soft_constraints_params(mass, damping_ratio, frequency, delta_time)`：返回 `(force_soft_factor, position_fix_factor)`。

- `PointConstraint<Obj>`
  - `PointConstraint::new(id, obj_id, fixed_point, move_point, config)`
  - `stretch_length()`：返回当前伸长向量。
  - 场景在预求解时会调用内部 `reset_params` 与 `solve`（无需手动）。

- `JoinConstraint<Obj>`（详见 `constraints/join.rs`）
  - 提供双元素约束，支持软硬配置。
  - 提供 `move_point_pair()`、`config()` 等访问器。

- `contact::ContactConstraint` 与 `contact_manifold::ContactConstraintManifold`
  - 管理碰撞接触对及其累计冲量。

## 11. 调试与工具 (`tools` 模块)

- `tools::snapshot::create_element_construct_code_snapshot(element)`：根据元素当前状态生成可复现的 Rust 构造代码字符串。
- `tools::drag::Draggable`
  - `on_mouse_down(scene)` / `on_mouse_move(scene, x, y)` / `on_mouse_up()`：实现简单的鼠标拖拽交互。
  - `mouse_point()`：返回当前鼠标位置。
- `tools::collision_view::CollisionStatusViewer`
  - `on_update(scene)`：刷新内部接触状态缓存。
  - `get_collision_infos()`：获取 `ContactInfos` 列表，调试用。

## 12. 算法 (`algo` 模块)

`algo::sort` 模块主要供内部使用，但对外可见的 trait 有助于自定义集合：

- `SortableCollection`（`pub(crate)`，默认不对外开放）：若需扩展，可参考 `ElementStore` 实现快速排序与插入排序组合策略。

## 13. 实用技巧

- **使用 `Scene::tick` 的 delta time**：内部限制在 `[1/60, 1/25]` 区间，避免跳帧导致数值不稳定。
- **约束绑定点**：创建约束时会自动在元素内部注册绑定点，删除约束时记得调用 `remove_point_constraint`/`remove_join_constraint`，以移除绑定引用。
- **睡眠模式**：当启用睡眠后，元素动能低于阈值并持续若干帧会自动停用，需外力唤醒时可调用 `scene.silent()` 或直接修改元素元数据。

## 14. 进一步阅读

示例目录 `crates/picea/examples/` 覆盖了积木堆叠、布料、桥梁、坑洞等场景。可参考 `stack.rs`、`bridge.rs` 等文件，了解多约束组合的实践方式。
