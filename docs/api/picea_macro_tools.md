---
title: picea-macro-tools API 文档
---

# picea-macro-tools 宏库

`picea-macro-tools` 为 `picea` 项目及其扩展提供一组派生宏与属性宏，帮助快速生成 Builder、字段访问器、Deref 实现以及 WebAssembly 配置映射。本节逐一介绍公开宏、支持的属性参数以及常见用法示例。

## 1. 派生宏总览

| 宏 | 目标 | 说明 |
| --- | --- | --- |
| `#[derive(Shape)]` | 结构体 | 自动实现碰撞相关 trait，使形状可直接参与物理引擎计算 |
| `#[derive(Deref)]` | 结构体 | 基于 `#[deref]` 标记的字段生成 `Deref`/`DerefMut` |
| `#[derive(Builder)]` | 结构体 | 生成 `*Builder` 构建器、默认值及链式设置方法 |
| `#[derive(Fields)]` | 结构体 | 依据字段属性生成读取、写入、可变引用等访问器 |
| `#[wasm_config(...)]` | 结构体 | 生成 WebAssembly 绑定结构及 `serde_wasm_bindgen` 映射 |

以下章节分别说明每个宏的行为与可选参数。

## 2. `#[derive(Shape)]`

`Shape` 派生宏会为目标结构体实现以下 trait：

- `crate::collision::Collider`
- `crate::element::SelfClone`

该宏假设目标结构体自身已经实现（或通过 `Fields`/`Deref` 间接提供）`GeometryTransformer`、`CenterPoint`、`EdgeIterable`、`NearestPoint`、`Projector`、`ComputeMomentOfInertia` 等 trait。

### 属性

- `#[inner]`：可与 `Fields` 配合，标记嵌套字段以导出内部访问器。

### 示例

```10:18:crates/picea/src/shape/rect.rs
#[derive(Clone, Debug, Shape, Deref, Fields)]
pub struct Rect {
    #[r]
    width: f32,
    #[r]
    height: f32,
    #[deref]
    inner: ConstPolygon<4>,
}
```

通过 `Shape` 与 `Deref` 联合使用，`Rect` 自动具备碰撞体能力，并将大部分实现委托给内部的 `ConstPolygon`。

## 3. `#[derive(Deref)]`

该宏依赖字段级属性 `#[deref]` 指定唯一的目标字段，生成 `core::ops::Deref` 与 `DerefMut` 实现。

### 使用要点

- 仅支持结构体。
- 必须恰好有一个字段带 `#[deref]`。
- 可结合 `Fields` 为外层结构体提供访问器，同时继承内层类型的方法。

## 4. `#[derive(Builder)]`

`Builder` 派生宏会为结构体生成同名加后缀 `Builder` 的构建器类型，提供默认实现、`new()` 构造方法、链式字段设置以及 `From<Builder>` 转换。

### 属性

- `#[default = <expr>]`：指定字段的默认值表达式。若缺失则调用 `Default::default()`。
- `#[builder(skip)]`：跳过为该字段生成链式 setter。
- `#[shared(skip)]`：与 `Fields` 配置一致，阻止在生成器和访问器中重复暴露字段。

### 生成结果

- `FooBuilder` 结构体包含与原结构体相同的字段。
- `impl Default for FooBuilder` 与 `impl Default for Foo` 会根据 `#[default]` 自动填充。
- `impl From<FooBuilder> for Foo`：方便在构建器与目标类型之间转换。

## 5. `#[derive(Fields)]`

`Fields` 宏根据字段属性生成访问器/修改器。默认情况下：

- 为原结构体添加只读方法（返回引用或原始值）。
- 可选生成可变引用方法、设置器或使用闭包的 reducer。

### 字段级属性

- `#[r]`：生成只读访问器。支持参数：
  - `vis(pub(crate))`：覆盖生成方法的可见性。
  - `copy`：强制返回值而非引用。
- `#[w]`：生成写访问能力。支持参数：
  - `vis(...)`：设置方法可见性。
  - `reducer`：生成 `set_<field>(FnOnce(T) -> T)`，用于基于旧值计算新值。
  - `set`：生成 `set_<field>(Into<T>)`，链式返回 `&mut Self`。
- `#[shared(skip)]`：完全跳过该字段。

### 结构体级属性

- 可在结构体上使用 `#[r(...)]` 或 `#[w(...)]` 设置默认访问策略，供未显式标注的字段继承。

### 示例

```85:95:crates/picea/src/element/mod.rs
#[derive(Fields)]
#[r]
pub struct Element<Data: Clone> {
    id: ID,
    #[w]
    meta: Meta,
    #[w]
    shape: Box<dyn ShapeTraitUnion>,
    bind_points: BTreeMap<u32, Vector>,
    data: Data,
}
```

上例展示了：

- 结构体级 `#[r]` 使未显式标注的字段默认生成 `fn field(&self) -> &T`。
- `#[w]` 为 `meta`、`shape` 生成 `set_meta`, `meta_mut` 等方法，便于运行时调整。

## 6. `#[wasm_config]` 属性宏

`wasm_config` 属性宏用于定义可导出到 WebAssembly 的配置结构。核心特性：

- 自动派生 `picea_macro_tools::Fields`（便于读写字段）。
- 自动派生 `Serialize`/`Deserialize`，支持通过 `serde_wasm_bindgen` 与 JS 交互。
- 生成 `Web<Struct>` 与 `OptionalWeb<Struct>` 类型别名（对应完整配置与部分配置）。
- 若提供 `bind = TypeName` 参数，会生成与 `picea::prelude::TypeName` 互转的实现，方便在 Rust 与 JS 之间共享构建器。

### 字段默认值

- 与 `Builder` 类似，支持在字段上使用 `#[default = <expr>]` 定义 JS 端缺省值。

### 示例

```83:95:crates/picea-web/src/common.rs
#[wasm_config(bind = Meta)]
pub(crate) struct Meta {
    #[default = 1.0]
    pub mass: FloatNum,
    #[default = true]
    pub is_fixed: bool,
    pub is_transparent: bool,
    pub velocity: Tuple2,
    #[default = 0.2]
    pub factor_friction: FloatNum,
    #[default = 1.]
    pub factor_restitution: FloatNum,
}
```

生成的辅助类型与实现包括：

- `WebMeta` / `OptionalWebMeta`：用于 JS 层类型提示与运行时校验。
- `impl From<&picea::prelude::Meta> for Meta` 与 `impl From<&Meta> for picea::prelude::MetaBuilder`（当配置包含 `bind` 时）。
- `impl Default for Meta`：按 `#[default]` 设置初始值。

## 7. 最佳实践

- **组合使用**：常见组合是 `#[derive(Fields, Builder)]`，先生成访问器，再通过构建器提供友好的初始化 API。
- **可见性控制**：通过 `vis(pub(crate))` 等参数精细化 getter/setter 暴露范围，避免内部字段泄漏。
- **与 `picea` 配合**：在自定义形状或约束对象上，推荐同时使用 `Shape` 和 `Fields`，并在场景初始化阶段利用 `Builder` 快速创建 `Meta` 或约束配置。
- **WebAssembly 场景**：结合 `wasm_config` 与 `serde_wasm_bindgen`，可以在 TypeScript 中获得强类型提示，同时保持 Rust 端构建逻辑一致。

如需进一步了解代码生成的具体实现，可直接查阅 `crates/macro-tools/src/` 目录下对应的宏实现源文件。
