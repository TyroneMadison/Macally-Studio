import { Suspense, useCallback, useEffect, useMemo, useRef, useState, type ComponentRef } from "react";
import { Canvas, useThree, type ThreeEvent } from "@react-three/fiber";
import { OrbitControls, RoundedBox, Text } from "@react-three/drei";
import { BackSide, MOUSE } from "three";
import fontUrl from "../assets/fonts/JetBrainsMono-Variable.ttf";
import type { Layout, LayoutKey } from "../lib/types";
import { clampView, YAW_LIMIT, ZOOM_MAX, ZOOM_MIN } from "../lib/view";

export type Keyboard3DProps = {
  layout: Layout;
  boundCodes: Set<string>;
  selected: string | null;
  onSelect: (code: string) => void;
  legends: "mac" | "win";
  yaw: number; // radians, -PI/2..PI/2
  zoom: number; // 1..1.5
  onViewChange: (v: { yaw: number; zoom: number }) => void;
};

const KEY_H = 0.3; // keycap height, world units (1 world unit = 1u)
const PLATE_T = 0.25;
const PLATE_MARGIN = 0.5;
const FN_GAP = 0.15; // the real board leaves a little air under the F row
const POLAR = 0.9; // locked tilt from straight down (~52 deg), the product-shot angle
const FOV = 32;
const GAP = 0.1; // space between neighbouring caps
const LIFT = 0.08; // caps float a little so the backlight shows between them

type Controls = ComponentRef<typeof OrbitControls>;

/** Top edge of a row: the F row is 0.6u tall, rows 1-5 are 1u. */
const rowTop = (row: number) => (row === 0 ? 0 : 0.6 + FN_GAP + (row - 1));

function measure(keys: LayoutKey[]) {
  let width = 0;
  let depth = 0;
  for (const k of keys) {
    width = Math.max(width, k.x + k.w);
    depth = Math.max(depth, rowTop(k.row) + k.h);
  }
  return { width, depth, halfSpan: width / 2 + PLATE_MARGIN };
}

/** Camera distance at which the plate spans the canvas width at 100% zoom. */
function fitDistance(halfSpan: number, aspect: number) {
  return (halfSpan / (Math.tan((FOV / 2) * (Math.PI / 180)) * aspect)) * 1.15;
}

function cameraPos(yaw: number, zoom: number, fit: number): [number, number, number] {
  const r = fit / zoom;
  return [r * Math.sin(POLAR) * Math.sin(yaw), r * Math.cos(POLAR), r * Math.sin(POLAR) * Math.cos(yaw)];
}

type KeyProps = {
  k: LayoutKey;
  legend: string;
  width: number;
  depth: number;
  bound: boolean;
  selected: boolean;
  onSelect: (code: string) => void;
};

function Key({ k, legend, width, depth, bound, selected, onSelect }: KeyProps) {
  const cap = useMemo<[number, number, number]>(() => [k.w - GAP, KEY_H, k.h - GAP], [k.w, k.h]);
  const onClick = useCallback(
    (e: ThreeEvent<MouseEvent>) => {
      e.stopPropagation();
      onSelect(k.code);
    },
    [k.code, onSelect],
  );
  const short = legend.length <= 2;
  const fontSize = short ? (k.h < 1 ? 0.2 : 0.3) : k.h < 1 ? 0.12 : 0.15;
  return (
    <group position={[k.x + k.w / 2 - width / 2, PLATE_T / 2 + KEY_H / 2, rowTop(k.row) + k.h / 2 - depth / 2]}>
      {/* backlight bleeding around the cap, like the real board's white LEDs: bright when bound */}
      <RoundedBox args={[k.w - 0.01, 0.02, k.h - 0.01]} radius={0.04} smoothness={2} position={[0, -KEY_H / 2 + 0.01, 0]}>
        <meshBasicMaterial color={bound ? "#FFFFFF" : "#2E2E33"} toneMapped={false} />
      </RoundedBox>
      <RoundedBox
        args={cap}
        radius={0.06}
        smoothness={4}
        position={[0, LIFT, 0]}
        onClick={onClick}
        onPointerOver={() => (document.body.style.cursor = "pointer")}
        onPointerOut={() => (document.body.style.cursor = "")}
      >
        <meshStandardMaterial
          color={selected ? "#1E2A44" : "#151517"}
          metalness={0.1}
          roughness={0.6}
          emissive="#4C8DFF"
          emissiveIntensity={selected ? 0.35 : 0}
        />
      </RoundedBox>
      {selected && (
        // blue outline: a slightly larger shell that only draws its back faces
        <RoundedBox args={[k.w - GAP + 0.08, KEY_H + 0.04, k.h - GAP + 0.08]} radius={0.08} smoothness={4} position={[0, LIFT, 0]}>
          <meshBasicMaterial color="#4C8DFF" side={BackSide} />
        </RoundedBox>
      )}
      {legend !== "" && (
        <Suspense fallback={null}>
          <Text
            font={fontUrl}
            fontSize={fontSize}
            maxWidth={k.w - 0.2}
            textAlign="center"
            color={bound || selected ? "#FFFFFF" : "#8A8A93"}
            outlineWidth={bound ? "12%" : 0}
            outlineBlur={bound ? "40%" : 0}
            outlineColor="#FFFFFF"
            outlineOpacity={0.55}
            anchorX="center"
            anchorY="middle"
            depthOffset={-1}
            position={[0, LIFT + KEY_H / 2 + 0.003, 0]}
            rotation={[-Math.PI / 2, 0, 0]}
          >
            {legend}
          </Text>
        </Suspense>
      )}
    </group>
  );
}

type RigProps = { yaw: number; zoom: number; halfSpan: number; onViewChange: Keyboard3DProps["onViewChange"] };

/** OrbitControls kept in two-way sync with the yaw/zoom props. Lives inside <Canvas>. */
function Rig({ yaw, zoom, halfSpan, onViewChange }: RigProps) {
  const aspect = useThree((s) => (s.size.width > 0 && s.size.height > 0 ? s.size.width / s.size.height : 1));
  const fit = fitDistance(halfSpan, aspect);
  const ref = useRef<Controls>(null);
  const applying = useRef(false);

  // props -> camera (sliders, Flip, Reset)
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const v = clampView({ yaw, zoom });
    const dist = fit / v.zoom;
    if (Math.abs(c.getAzimuthalAngle() - v.yaw) < 1e-3 && Math.abs(c.getDistance() - dist) < 1e-3) return;
    applying.current = true;
    c.setAzimuthalAngle(v.yaw); // three-stdlib OrbitControls (drei) has this; vanilla three's does not
    c.object.position.sub(c.target).setLength(dist).add(c.target); // three-stdlib has no setDistance
    c.update();
    applying.current = false;
  }, [yaw, zoom, fit]);

  // camera -> props (right-drag, wheel, middle-drag)
  const onChange = useCallback(() => {
    const c = ref.current;
    if (!c || applying.current) return;
    onViewChange(clampView({ yaw: c.getAzimuthalAngle(), zoom: fit / c.getDistance() }));
  }, [fit, onViewChange]);

  return (
    <OrbitControls
      ref={ref}
      // LEFT is left out on purpose: left click only selects keys
      mouseButtons={{ RIGHT: MOUSE.ROTATE, MIDDLE: MOUSE.DOLLY }}
      enablePan={false}
      enableDamping={false}
      enableZoom
      zoomSpeed={1}
      minPolarAngle={POLAR}
      maxPolarAngle={POLAR}
      minAzimuthAngle={-YAW_LIMIT}
      maxAzimuthAngle={YAW_LIMIT}
      minDistance={fit / ZOOM_MAX}
      maxDistance={fit / ZOOM_MIN}
      onChange={onChange}
    />
  );
}

export default function Keyboard3D({
  layout,
  boundCodes,
  selected,
  onSelect,
  legends,
  yaw,
  zoom,
  onViewChange,
}: Keyboard3DProps) {
  const { width, depth, halfSpan } = useMemo(() => measure(layout.keys), [layout]);
  const square = fitDistance(halfSpan, 1);
  // initial camera only; Rig fits it to the canvas aspect and keeps it in sync afterwards
  const [initial] = useState(() => cameraPos(yaw, zoom, square));

  return (
    <Canvas
      dpr={[1, 2]}
      camera={{ fov: FOV, near: 0.1, far: square * 4, position: initial }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <ambientLight intensity={0.6} />
      <directionalLight position={[6, 10, 6]} intensity={1.4} />
      <directionalLight position={[-8, 6, -4]} intensity={0.5} />
      {/* plate: space gray aluminium */}
      <RoundedBox args={[width + PLATE_MARGIN * 2, PLATE_T, depth + PLATE_MARGIN * 2]} radius={0.12} smoothness={6}>
        <meshStandardMaterial color="#6A6C73" metalness={0.35} roughness={0.45} />
      </RoundedBox>
      {layout.keys.map((k) => (
        <Key
          key={k.code}
          k={k}
          legend={legends === "mac" ? k.mac : k.win}
          width={width}
          depth={depth}
          bound={boundCodes.has(k.code)}
          selected={k.code === selected}
          onSelect={onSelect}
        />
      ))}
      <Rig yaw={yaw} zoom={zoom} halfSpan={halfSpan} onViewChange={onViewChange} />
    </Canvas>
  );
}
