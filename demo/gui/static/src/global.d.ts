/**
 * Global type declarations for external libraries loaded via CDN
 */

// D3.js global namespace declarations (loaded from CDN)
declare namespace d3 {
  // Selection
  interface Selection<GElement extends Element, Datum, PElement extends Element, PDatum> {
    select<DescElement extends Element>(selector: string): Selection<DescElement, Datum, PElement, PDatum>;
    selectAll<DescElement extends Element>(selector: string): Selection<DescElement, Datum, PElement, PDatum>;
    append<K extends keyof SVGElementTagNameMap>(type: K): Selection<SVGElementTagNameMap[K], Datum, GElement, Datum>;
    append(type: string): Selection<Element, Datum, GElement, Datum>;
    attr(name: string, value: string | number | boolean | null | ((d: Datum, i: number) => string | number | boolean | null)): this;
    attr(name: string): string;
    style(name: string, value: string | null, priority?: string): this;
    classed(names: string, value: boolean): this;
    text(value: string | number | null | ((d: Datum) => string)): this;
    html(value: string): this;
    data<NewDatum>(data: NewDatum[]): Selection<GElement, NewDatum, PElement, PDatum>;
    enter(): Selection<GElement, Datum, PElement, PDatum>;
    exit(): Selection<GElement, Datum, PElement, PDatum>;
    remove(): this;
    node(): GElement | null;
    filter(selector: string | ((datum: unknown, index: number) => boolean)): this;
    datum(): Datum;
    call<Args extends unknown[]>(func: (selection: this, ...args: Args) => void, ...args: Args): this;
    on(typenames: string, listener: ((event: Event, d: Datum) => void) | null): this;
    transition(): Transition<GElement, Datum, PElement, PDatum>;
  }

  interface Transition<GElement extends Element, Datum, PElement extends Element, PDatum> {
    attr(name: string, value: string | number): this;
    duration(ms: number): this;
    call<Args extends unknown[]>(func: (transition: this, ...args: Args) => void, ...args: Args): this;
  }

  function select<GElement extends Element>(selector: string | GElement): Selection<GElement, unknown, null, undefined>;

  // Zoom
  interface ZoomBehavior<ZoomRefElement extends Element, Datum> {
    (selection: Selection<ZoomRefElement, Datum, Element, unknown>): void;
    scaleExtent(extent: [number, number]): this;
    on(typenames: string, listener: (event: D3ZoomEvent<ZoomRefElement, Datum>) => void): this;
    transform(selection: Selection<ZoomRefElement, Datum, Element, unknown> | Transition<ZoomRefElement, Datum, Element, unknown>, transform: ZoomTransform): void;
    scaleBy(selection: Selection<ZoomRefElement, Datum, Element, unknown> | Transition<ZoomRefElement, Datum, Element, unknown>, k: number): void;
  }

  interface ZoomTransform {
    k: number;
    x: number;
    y: number;
    toString(): string;
    translate(x: number, y: number): ZoomTransform;
    scale(k: number): ZoomTransform;
  }

  interface D3ZoomEvent<ZoomRefElement extends Element, Datum> {
    type: string;
    target: ZoomBehavior<ZoomRefElement, Datum>;
    transform: ZoomTransform;
    sourceEvent: Event;
  }

  function zoom<ZoomRefElement extends Element, Datum>(): ZoomBehavior<ZoomRefElement, Datum>;
  const zoomIdentity: ZoomTransform;

  // Drag
  interface DragBehavior<GElement extends Element, Datum, Subject> {
    (selection: Selection<GElement, Datum, Element, unknown>): void;
    on(typenames: string, listener: ((event: D3DragEvent<GElement, Datum, Subject>, d: Datum) => void) | null): this;
  }

  interface D3DragEvent<GElement extends Element, Datum, Subject> {
    type: string;
    subject: Subject;
    x: number;
    y: number;
    dx: number;
    dy: number;
    active: number;
    sourceEvent: Event;
  }

  function drag<GElement extends Element, Datum>(): DragBehavior<GElement, Datum, Datum>;

  // Force Simulation
  interface Simulation<NodeDatum, LinkDatum> {
    nodes(nodes: NodeDatum[]): this;
    force(name: string, force?: Force<NodeDatum, LinkDatum> | null): this;
    alphaTarget(target: number): this;
    restart(): this;
    stop(): this;
    on(typenames: string, listener: () => void): this;
  }

  interface Force<NodeDatum, LinkDatum> {
    (alpha: number): void;
  }

  interface ForceLink<NodeDatum, LinkDatum> extends Force<NodeDatum, LinkDatum> {
    id(id: (d: NodeDatum) => string): this;
    distance(distance: number): this;
    (links: LinkDatum[]): this;
  }

  interface ForceManyBody<NodeDatum> extends Force<NodeDatum, never> {
    strength(strength: number): this;
  }

  interface ForceCenter<NodeDatum> extends Force<NodeDatum, never> {}

  interface ForceCollide<NodeDatum> extends Force<NodeDatum, never> {
    radius(radius: number | ((d: NodeDatum) => number)): this;
  }

  function forceSimulation<NodeDatum>(nodes?: NodeDatum[]): Simulation<NodeDatum, never>;
  function forceLink<NodeDatum, LinkDatum>(links?: LinkDatum[]): ForceLink<NodeDatum, LinkDatum>;
  function forceManyBody<NodeDatum>(): ForceManyBody<NodeDatum>;
  function forceCenter<NodeDatum>(x?: number, y?: number): ForceCenter<NodeDatum>;
  function forceCollide<NodeDatum>(): ForceCollide<NodeDatum>;
}
