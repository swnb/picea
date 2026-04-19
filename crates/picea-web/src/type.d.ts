type Vector = { x: number; y: number }
type Point = { x: number; y: number }
type Meta = {
  mass: number
  isFixed: boolean
  isTransparent: boolean
  angle: number
  velocity: { x: number; y: number }
  factorRestitution: number
  factorFriction: number
}
type MetaPartial = Partial<Meta>

type JoinConstraintConfig = {
  distance: number
  dampingRatio: number
  frequency: number
  hard: boolean
}
type JoinConstraintConfigPartial = Partial<JoinConstraintConfig>

type Shape = {
  readonly id: number
  readonly centerPoint: Point
} & (
  | {
      readonly shapeType: "circle"
      radius: number
    }
  | {
      readonly shapeType: "polygon"
      vertices: () => Point[]
    }
)

interface WebScene {
  /** Legacy-compatible wrapper: invalid numbers/meta return 0. Width and height must be greater than 0. */
  createRect(
    topLeftX: number,
    topRightY: number,
    width: number,
    height: number,
    metaData?: MetaPartial
  ): number
  /** Throws a JS error value when numbers/meta are invalid. Width and height must be greater than 0. */
  tryCreateRect(
    topLeftX: number,
    topRightY: number,
    width: number,
    height: number,
    metaData?: MetaPartial
  ): number

  /** Legacy-compatible wrapper: invalid numbers/meta return 0. Radius must be greater than 0. */
  createCircle(
    centerPointX: number,
    centerPointY: number,
    radius: number,
    metaData?: MetaPartial
  ): number
  /** Throws a JS error value when numbers/meta are invalid. Radius must be greater than 0. */
  tryCreateCircle(
    centerPointX: number,
    centerPointY: number,
    radius: number,
    metaData?: MetaPartial
  ): number

  /** Legacy-compatible wrapper: invalid numbers/meta return 0. Radius must be greater than 0; edge count must be an integer from 3 to 1024. */
  createRegularPolygon(
    x: number,
    y: number,
    edgeCount: number,
    radius: number,
    metaData?: MetaPartial
  ): number
  /** Throws a JS error value when numbers/meta are invalid. Radius must be greater than 0; edge count must be an integer from 3 to 1024. */
  tryCreateRegularPolygon(
    x: number,
    y: number,
    edgeCount: number,
    radius: number,
    metaData?: MetaPartial
  ): number

  /** Legacy-compatible wrapper: invalid gravity is ignored. */
  setGravity(gravity: Vector): void
  /** Throws a JS error value when gravity is not a finite Vector. */
  trySetGravity(gravity: Vector): void

  /** Legacy-compatible wrapper: invalid points/meta return 0. */
  createPolygon(vertices: Point[], metaData?: MetaPartial): number
  /** Throws a JS error value when points/meta are invalid. */
  tryCreatePolygon(vertices: Point[], metaData?: MetaPartial): number

  /** Legacy-compatible wrapper: invalid points/meta return 0. */
  createLine(startPoint: Point, endPoint: Point, metaData?: MetaPartial): number
  /** Throws a JS error value when points/meta are invalid. */
  tryCreateLine(startPoint: Point, endPoint: Point, metaData?: MetaPartial): number

  /** Legacy-compatible wrapper: invalid meta or missing element is ignored. */
  updateElementMeta(elementId: number, metaData: MetaPartial): void
  /** Throws a JS error value when meta is invalid or the element is missing. */
  tryUpdateElementMeta(elementId: number, metaData: MetaPartial): void

  /** Legacy-compatible wrapper: missing source returns undefined; invalid meta falls back to defaults. */
  cloneElement(elementId: number, metaData?: MetaPartial): number | undefined
  /** Throws a JS error value when the source is missing or meta is invalid. */
  tryCloneElement(elementId: number, metaData?: MetaPartial): number

  /** Legacy-compatible wrapper: invalid input returns undefined. */
  createPointConstraint(
    elementId: number,
    elementPoint: Point,
    fixedPoint: Point,
    constraintConfig: JoinConstraintConfigPartial
  ): PointConstraint | undefined
  /** Throws a JS error value when input is invalid or the element is missing. */
  tryCreatePointConstraint(
    elementId: number,
    elementPoint: Point,
    fixedPoint: Point,
    constraintConfig: JoinConstraintConfigPartial
  ): PointConstraint

  /** Legacy-compatible wrapper: invalid input returns undefined. */
  createJoinConstraint(
    elementAId: number,
    elementAPoint: Point,
    elementBId: number,
    elementBPoint: Point,
    constraintConfig: JoinConstraintConfigPartial
  ): JoinConstraint | undefined
  /** Throws a JS error value when input is invalid or either element is missing. */
  tryCreateJoinConstraint(
    elementAId: number,
    elementAPoint: Point,
    elementBId: number,
    elementBPoint: Point,
    constraintConfig: JoinConstraintConfigPartial
  ): JoinConstraint

  /** Legacy-compatible wrapper: missing element returns null. */
  getKinetic(elementId: number): unknown | null
  /** Throws a JS error value when the element is missing. */
  tryGetKinetic(elementId: number): unknown

  /** Legacy-compatible wrapper: missing elements or unsupported circle/arc edges return an empty array. */
  getElementVertices(elementId: number): Point[]
  /** Throws a JS error value when the element is missing or its edges cannot be represented as vertices. */
  tryGetElementVertices(elementId: number): Point[]

  /** Callback errors are caught and ignored by the Rust scene. Unsupported arc edges are skipped without stopping iteration. */
  forEachElement: (callback: (shape: Shape) => void) => void
  /** Callback errors are caught and ignored by the Rust scene. */
  registerElementPositionUpdateCallback(
    callback: (
      id: number,
      translate: { x: number; y: number },
      rotation: number
    ) => void
  ): number
}

interface PointConstraint {
  /** Legacy-compatible wrapper: invalid point or disposed constraint is ignored. */
  updateMovePoint(point: Point): void
  /** Throws a JS error value when point is invalid or the constraint is gone. */
  tryUpdateMovePoint(point: Point): void
  /** Legacy-compatible wrapper: invalid config or disposed constraint is ignored. */
  updateConfig(config: JoinConstraintConfigPartial): void
  /** Throws a JS error value when config is invalid or the constraint is gone. */
  tryUpdateConfig(config: JoinConstraintConfigPartial): void
}

interface JoinConstraint {
  /** Legacy-compatible wrapper: invalid config or disposed constraint is ignored. */
  updateConfig(config: JoinConstraintConfigPartial): void
  /** Throws a JS error value when config is invalid or the constraint is gone. */
  tryUpdateConfig(config: JoinConstraintConfigPartial): void
}

/** Legacy-compatible wrapper: invalid point input returns false. */
declare function isPointValidAddIntoPolygon(point: Point, vertices: Point[]): boolean
/** Throws a JS error value when point input is invalid. */
declare function tryIsPointValidAddIntoPolygon(point: Point, vertices: Point[]): boolean
