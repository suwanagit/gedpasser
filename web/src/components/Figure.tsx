import type { Figure as FigureData } from '../api/types/Figure'
import type { FigureView } from '../api/types/FigureView'
import type { Pt } from '../api/types/Pt'
import type { Seg } from '../api/types/Seg'

/** Draws a figure (graph, number line, chart or shape) as an SVG. */
export function Figure({ view }: { view: FigureView }) {
  const f = view.figure
  return (
    <figure className="figure" aria-label={describe(f)}>
      {f.kind === 'plane' && <Plane f={f} curves={view.curves} />}
      {f.kind === 'number_line' && <NumberLine f={f} />}
      {f.kind === 'bars' && <Bars f={f} />}
      {f.kind === 'dots' && <Dots f={f} />}
      {f.kind === 'box' && <BoxPlot f={f} />}
      {f.kind === 'shape' && <Shape f={f} />}
    </figure>
  )
}

function describe(f: FigureData): string {
  switch (f.kind) {
    case 'plane':
      return 'Graph on a coordinate plane'
    case 'number_line':
      return 'Number line'
    case 'bars':
      return 'Bar graph'
    case 'dots':
      return 'Dot plot'
    case 'box':
      return 'Box plot'
    case 'shape':
      return 'Diagram'
  }
}

/** A tidy step (1, 2, 5 × 10ⁿ) for about `target` ticks across `range`. */
function niceStep(range: number, target = 10): number {
  const raw = range / target
  const p = Math.pow(10, Math.floor(Math.log10(raw)))
  const m = raw / p
  return (m <= 1 ? 1 : m <= 2 ? 2 : m <= 5 ? 5 : 10) * p
}

function ticks(min: number, max: number, step: number): number[] {
  const out: number[] = []
  for (let v = Math.ceil(min / step) * step; v <= max + step / 1000; v += step) out.push(Math.round(v * 1e6) / 1e6)
  return out
}

const fmt = (v: number) => String(Math.round(v * 1000) / 1000).replace('-', '−')

// ── Coordinate plane ──────────────────────────────────────────────────────

type PlaneF = Extract<FigureData, { kind: 'plane' }>

function Plane({ f, curves }: { f: PlaneF; curves: FigureView['curves'] }) {
  const xr = f.x_max - f.x_min
  const yr = f.y_max - f.y_min
  const W = 320
  const pad = 26
  // Equal scale on both axes so slopes and right angles look true.
  const scale = (W - 2 * pad) / xr
  const H = Math.min(Math.max(yr * scale + 2 * pad, 160), 420)
  const s = Math.min(scale, (H - 2 * pad) / yr)
  const ox = (W - xr * s) / 2
  const oy = (H - yr * s) / 2
  const X = (x: number) => ox + (x - f.x_min) * s
  const Y = (y: number) => H - oy - (y - f.y_min) * s
  const xs = niceStep(xr)
  const ys = niceStep(yr)
  const ax = f.x_min <= 0 && f.x_max >= 0 ? 0 : f.x_min
  const ay = f.y_min <= 0 && f.y_max >= 0 ? 0 : f.y_min
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="fig-svg" role="img">
      <g className="fig-grid">
        {ticks(f.x_min, f.x_max, xs).map((v) => (
          <line key={`gx${v}`} x1={X(v)} x2={X(v)} y1={Y(f.y_min)} y2={Y(f.y_max)} />
        ))}
        {ticks(f.y_min, f.y_max, ys).map((v) => (
          <line key={`gy${v}`} y1={Y(v)} y2={Y(v)} x1={X(f.x_min)} x2={X(f.x_max)} />
        ))}
      </g>
      <g className="fig-axis">
        <line x1={X(f.x_min)} x2={X(f.x_max)} y1={Y(ay)} y2={Y(ay)} />
        <line y1={Y(f.y_min)} y2={Y(f.y_max)} x1={X(ax)} x2={X(ax)} />
      </g>
      <g className="fig-ticklabels">
        {ticks(f.x_min, f.x_max, xs)
          .filter((v) => v !== ax)
          .map((v) => (
            <text key={`tx${v}`} x={X(v)} y={Y(ay) + 12} textAnchor="middle">
              {fmt(v)}
            </text>
          ))}
        {ticks(f.y_min, f.y_max, ys)
          .filter((v) => v !== ay)
          .map((v) => (
            <text key={`ty${v}`} x={X(ax) - 4} y={Y(v) + 3} textAnchor="end">
              {fmt(v)}
            </text>
          ))}
      </g>
      {f.x_label && (
        <text className="fig-axis-label" x={X(f.x_max)} y={Y(ay) - 6} textAnchor="end">
          {f.x_label}
        </text>
      )}
      {f.y_label && (
        <text className="fig-axis-label" x={X(ax) + 6} y={Y(f.y_max) + 10}>
          {f.y_label}
        </text>
      )}
      {curves.map((pieces, ci) =>
        pieces.map((line, li) => (
          <polyline
            key={`c${ci}-${li}`}
            className={`fig-curve c${ci % 3}`}
            points={line.map(([x, y]) => `${X(x)},${Y(y)}`).join(' ')}
          />
        )),
      )}
      <Segments segs={f.segments} X={X} Y={Y} />
      <Points pts={f.points} X={X} Y={Y} />
    </svg>
  )
}

function Segments({ segs, X, Y }: { segs: Seg[]; X: (v: number) => number; Y: (v: number) => number }) {
  return (
    <g>
      {segs.map((sg, i) => {
        const [x1, y1] = [X(sg.from[0]), Y(sg.from[1])]
        const [x2, y2] = [X(sg.to[0]), Y(sg.to[1])]
        return (
          <g key={i}>
            <line className={`fig-seg${sg.dashed ? ' dashed' : ''}`} x1={x1} y1={y1} x2={x2} y2={y2} />
            {sg.label && (
              <text className="fig-label" x={(x1 + x2) / 2 + 4} y={(y1 + y2) / 2 - 4}>
                {sg.label}
              </text>
            )}
          </g>
        )
      })}
    </g>
  )
}

function Points({ pts, X, Y }: { pts: Pt[]; X: (v: number) => number; Y: (v: number) => number }) {
  return (
    <g>
      {pts.map((p, i) => (
        <g key={i}>
          <circle className="fig-point" cx={X(p.x)} cy={Y(p.y)} r={3.5} />
          {p.label && (
            <text className="fig-label" x={X(p.x) + 6} y={Y(p.y) - 6}>
              {p.label}
            </text>
          )}
        </g>
      ))}
    </g>
  )
}

// ── Number line ──────────────────────────────────────────────────────────

type LineF = Extract<FigureData, { kind: 'number_line' }>

function NumberLine({ f }: { f: LineF }) {
  const W = 320
  const H = 70
  const pad = 18
  const X = (x: number) => pad + ((x - f.min) / (f.max - f.min)) * (W - 2 * pad)
  const y = 32
  const step = niceStep(f.max - f.min, 12)
  const r = f.ray
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="fig-svg" role="img">
      <defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" className="fig-arrowhead" />
        </marker>
      </defs>
      <line className="fig-axis-line" x1={pad - 10} x2={W - pad + 10} y1={y} y2={y} markerStart="url(#arrow)" markerEnd="url(#arrow)" />
      {ticks(f.min, f.max, step).map((v) => (
        <g key={v} className="fig-ticklabels">
          <line className="fig-tick" x1={X(v)} x2={X(v)} y1={y - 5} y2={y + 5} />
          <text x={X(v)} y={y + 20} textAnchor="middle">
            {fmt(v)}
          </text>
        </g>
      ))}
      {r && (
        <line
          className="fig-ray"
          x1={X(r.from)}
          x2={r.dir === 'right' ? W - pad + 8 : pad - 8}
          y1={y}
          y2={y}
          markerEnd="url(#arrow)"
        />
      )}
      {r && <circle className={r.open ? 'fig-open' : 'fig-closed'} cx={X(r.from)} cy={y} r={5} />}
      {f.points.map((p, i) => (
        <g key={i}>
          <circle className={p.open ? 'fig-open' : 'fig-closed'} cx={X(p.x)} cy={y} r={5} />
          {p.label && (
            <text className="fig-label" x={X(p.x)} y={y - 11} textAnchor="middle">
              {p.label}
            </text>
          )}
        </g>
      ))}
    </svg>
  )
}

// ── Bar graph ────────────────────────────────────────────────────────────

type BarsF = Extract<FigureData, { kind: 'bars' }>

function Bars({ f }: { f: BarsF }) {
  const W = 320
  const H = 210
  const left = 44
  const bottom = 26
  const top = 14
  const max = Math.max(...f.bars.map((b) => b.value), 1)
  const step = niceStep(max, 5)
  const yMax = Math.ceil(max / step) * step
  const Y = (v: number) => H - bottom - (v / yMax) * (H - bottom - top)
  const bw = (W - left - 10) / f.bars.length
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="fig-svg" role="img">
      {ticks(0, yMax, step).map((v) => (
        <g key={v}>
          <line className="fig-gridline" x1={left} x2={W - 6} y1={Y(v)} y2={Y(v)} />
          <text className="fig-ticklabels" x={left - 5} y={Y(v) + 3} textAnchor="end">
            {fmt(v)}
          </text>
        </g>
      ))}
      {f.bars.map((b, i) => (
        <g key={b.label}>
          <rect className="fig-bar" x={left + i * bw + bw * 0.15} width={bw * 0.7} y={Y(b.value)} height={Y(0) - Y(b.value)} />
          <text className="fig-ticklabels" x={left + i * bw + bw / 2} y={H - 9} textAnchor="middle">
            {b.label}
          </text>
        </g>
      ))}
      <line className="fig-axis-line" x1={left} x2={W - 6} y1={Y(0)} y2={Y(0)} />
      {f.y_label && (
        <text className="fig-axis-label" x={left} y={9}>
          {f.y_label}
        </text>
      )}
    </svg>
  )
}

// ── Dot plot ─────────────────────────────────────────────────────────────

type DotsF = Extract<FigureData, { kind: 'dots' }>

function Dots({ f }: { f: DotsF }) {
  const xs = f.counts.map((c) => c.x)
  const min = Math.min(...xs)
  const max = Math.max(...xs)
  const tallest = Math.max(...f.counts.map((c) => c.count), 1)
  const W = 320
  const r = 6
  const H = 50 + tallest * (2 * r + 3)
  const pad = 24
  const X = (x: number) => (max === min ? W / 2 : pad + ((x - min) / (max - min)) * (W - 2 * pad))
  const base = H - 30
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="fig-svg" role="img">
      <line className="fig-axis-line" x1={pad - 12} x2={W - pad + 12} y1={base} y2={base} />
      {f.counts.map((c) => (
        <g key={c.x}>
          <line className="fig-tick" x1={X(c.x)} x2={X(c.x)} y1={base} y2={base + 5} />
          <text className="fig-ticklabels" x={X(c.x)} y={base + 17} textAnchor="middle">
            {fmt(c.x)}
          </text>
          {Array.from({ length: c.count }, (_, k) => (
            <circle key={k} className="fig-dot" cx={X(c.x)} cy={base - r - 2 - k * (2 * r + 3)} r={r} />
          ))}
        </g>
      ))}
      {f.label && (
        <text className="fig-axis-label" x={W / 2} y={H - 2} textAnchor="middle">
          {f.label}
        </text>
      )}
    </svg>
  )
}

// ── Box plot ─────────────────────────────────────────────────────────────

type BoxF = Extract<FigureData, { kind: 'box' }>

function BoxPlot({ f }: { f: BoxF }) {
  const W = 320
  const H = 100
  const pad = 20
  const X = (x: number) => pad + ((x - f.axis_min) / (f.axis_max - f.axis_min)) * (W - 2 * pad)
  const mid = 38
  const half = 16
  const step = niceStep(f.axis_max - f.axis_min, 10)
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="fig-svg" role="img">
      <line className="fig-seg" x1={X(f.min)} x2={X(f.q1)} y1={mid} y2={mid} />
      <line className="fig-seg" x1={X(f.q3)} x2={X(f.max)} y1={mid} y2={mid} />
      <line className="fig-seg" x1={X(f.min)} x2={X(f.min)} y1={mid - 8} y2={mid + 8} />
      <line className="fig-seg" x1={X(f.max)} x2={X(f.max)} y1={mid - 8} y2={mid + 8} />
      <rect className="fig-box" x={X(f.q1)} width={X(f.q3) - X(f.q1)} y={mid - half} height={2 * half} />
      <line className="fig-median" x1={X(f.median)} x2={X(f.median)} y1={mid - half} y2={mid + half} />
      <line className="fig-axis-line" x1={pad} x2={W - pad} y1={70} y2={70} />
      {ticks(f.axis_min, f.axis_max, step).map((v) => (
        <g key={v}>
          <line className="fig-tick" x1={X(v)} x2={X(v)} y1={70} y2={75} />
          <text className="fig-ticklabels" x={X(v)} y={87} textAnchor="middle">
            {fmt(v)}
          </text>
        </g>
      ))}
      {f.label && (
        <text className="fig-axis-label" x={W / 2} y={99} textAnchor="middle">
          {f.label}
        </text>
      )}
    </svg>
  )
}

// ── Shapes (geometry) ────────────────────────────────────────────────────

type ShapeF = Extract<FigureData, { kind: 'shape' }>

function Shape({ f }: { f: ShapeF }) {
  // Fit everything into the box with one scale (no distortion).
  const xs: number[] = []
  const ys: number[] = []
  f.polygons.forEach((p) => p.points.forEach((q) => (xs.push(q.x), ys.push(q.y))))
  f.circles.forEach((c) => (xs.push(c.x - c.r, c.x + c.r), ys.push(c.y - c.r, c.y + c.r)))
  f.segments.forEach((s) => (xs.push(s.from[0], s.to[0]), ys.push(s.from[1], s.to[1])))
  f.labels.forEach((l) => (xs.push(l.x), ys.push(l.y)))
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)]
  const W = 320
  const pad = 34
  const s = Math.min((W - 2 * pad) / Math.max(x1 - x0, 1e-6), 240 / Math.max(y1 - y0, 1e-6))
  const H = (y1 - y0) * s + 2 * pad
  const X = (x: number) => pad + (x - x0) * s + ((W - 2 * pad) - (x1 - x0) * s) / 2
  const Y = (y: number) => H - pad - (y - y0) * s
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="fig-svg" role="img">
      {f.circles.map((c, i) => (
        <g key={`c${i}`}>
          <circle className="fig-shape" cx={X(c.x)} cy={Y(c.y)} r={c.r * s} />
          <circle className="fig-point" cx={X(c.x)} cy={Y(c.y)} r={2.5} />
          {c.radius_label && (
            <>
              <line className="fig-seg" x1={X(c.x)} y1={Y(c.y)} x2={X(c.x + c.r)} y2={Y(c.y)} />
              <text className="fig-label" x={X(c.x + c.r / 2)} y={Y(c.y) - 6} textAnchor="middle">
                {c.radius_label}
              </text>
            </>
          )}
        </g>
      ))}
      {f.polygons.map((p, i) => {
        const pts = p.points.map((q) => [X(q.x), Y(q.y)] as const)
        const cx = pts.reduce((a, q) => a + q[0], 0) / pts.length
        const cy = pts.reduce((a, q) => a + q[1], 0) / pts.length
        return (
          <g key={`p${i}`}>
            <polygon className="fig-shape" points={pts.map((q) => q.join(',')).join(' ')} />
            {p.side_labels.map((label, k) => {
              if (!label) return null
              const a = pts[k]
              const b = pts[(k + 1) % pts.length]
              const mx = (a[0] + b[0]) / 2
              const my = (a[1] + b[1]) / 2
              // Push the label outward, away from the shape's center.
              const dx = mx - cx
              const dy = my - cy
              const len = Math.hypot(dx, dy) || 1
              return (
                <text key={k} className="fig-label" x={mx + (dx / len) * 14} y={my + (dy / len) * 14 + 4} textAnchor="middle">
                  {label}
                </text>
              )
            })}
            {p.right_angles.map((k) => {
              const v = pts[k]
              const a = pts[(k + pts.length - 1) % pts.length]
              const b = pts[(k + 1) % pts.length]
              const u = (q: readonly [number, number]) => {
                const l = Math.hypot(q[0] - v[0], q[1] - v[1]) || 1
                return [((q[0] - v[0]) / l) * 10, ((q[1] - v[1]) / l) * 10]
              }
              const [ua, ub] = [u(a), u(b)]
              const d = `M ${v[0] + ua[0]} ${v[1] + ua[1]} L ${v[0] + ua[0] + ub[0]} ${v[1] + ua[1] + ub[1]} L ${v[0] + ub[0]} ${v[1] + ub[1]}`
              return <path key={`r${k}`} className="fig-right-angle" d={d} />
            })}
          </g>
        )
      })}
      <Segments segs={f.segments} X={X} Y={Y} />
      {f.labels.map((l, i) => (
        <text key={`l${i}`} className="fig-label" x={X(l.x)} y={Y(l.y)} textAnchor="middle">
          {l.label}
        </text>
      ))}
    </svg>
  )
}
