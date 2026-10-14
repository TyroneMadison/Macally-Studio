import { Suspense, useCallback, useEffect, useMemo, useRef, useState, type ComponentRef } from "react";
import { Canvas, useThree, type ThreeEvent } from "@react-three/fiber";
import { Environment, Lightformer, OrbitControls, RoundedBox, Text } from "@react-three/drei";
import { BackSide, MeshBasicMaterial, MOUSE } from "three";
import fontUrl from "../assets/fonts/JetBrainsMono-Variable.ttf";
import type { Lighting } from "../lib/lighting";
import type { Layout, LayoutKey } from "../lib/types";
import {
  cameraPos,
  clampView,
  fitDistance,
  FOV,
  PITCH_MAX,
  PITCH_MIN,
  wrapAngle,
  ZOOM_MAX,
  ZOOM_MIN,
  type View3D,
} from "../lib/view";

export type Keyboard3DProps = {
  layout: Layout;
  boundCodes: Set<string>;
  selected: string | null;
  onSelect: (code: string) => void;
  legends: "mac" | "win";
  yaw: number; // radians, a full turn
  pitch: number; // radians from straight down, PITCH_MIN..PITCH_MAX
  zoom: number; // 1..1.5
  lighting: Lighting;
  onViewChange: (v: View3D) => void;
};

const KEY_H = 0.3; // keycap height, world units (1 world unit = 1u)
const PLATE_T = 0.25;
const PLATE_MARGIN = 0.5;
const FN_GAP = 0.15; // the real board leaves a little air under the F row
const GAP = 0.1; // space between neighbouring caps
const LIFT = 0.08; // caps float a little so the backlight shows between them
// per backlight level (Off, Low, Med, High): LED bleed under bound caps and the glow around their legends
const BLEED = ["#2E2E33", "#77777D", "#B9B9BD", "#FFFFFF"];
const GLOW = [0, 0.3, 0.45, 0.6];
// legends skip tone mapping so white prints read as white, not filmic gray (troika derives one per text)
const LEGEND = new MeshBasicMaterial({ toneMapped: false });

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
  // half the plate's diagonal, keycap tops included: the camera keeps this sphere in frame at any angle
  const radius = Math.hypot(width / 2 + PLATE_MARGIN, depth / 2 + PLATE_MARGIN, PLATE_T / 2 + LIFT + KEY_H);
  return { width, depth, radius };
}

type KeyProps = {
  k: LayoutKey;
  legend: string;
  width: number;
  depth: number;
  bound: boolean;
  selected: boolean;
  backlight: number;
  onSelect: (code: string) => void;
};

function Key({ k, legend, width, depth, bound, selected, backlight, onSelect }: KeyProps) {
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
  const glow = bound ? GLOW[backlight] : 0;
  // printed legends are near-white, lit ones pure white; with the backlight off they sit in dim gray
  const ink = selected || glow ? "#FFFFFF" : backlight ? "#E4E4E8" : "#9A9AA2";
  return (
    <group position={[k.x + k.w / 2 - width / 2, PLATE_T / 2 + KEY_H / 2, rowTop(k.row) + k.h / 2 - depth / 2]}>
      {/* backlight bleeding around the cap, like the real board's white LEDs: bright when bound */}
      <RoundedBox args={[k.w - 0.01, 0.02, k.h - 0.01]} radius={0.04} smoothness={2} position={[0, -KEY_H / 2 + 0.01, 0]}>
        <meshBasicMaterial color={BLEED[bound ? backlight : 0]} toneMapped={false} />
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
          color={selected ? "#22304D" : "#1D1D21"}
          metalness={0.1}
          roughness={0.55}
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
            material={LEGEND}
            fontSize={fontSize}
            maxWidth={k.w - 0.2}
            textAlign="center"
            color={ink}
            // lit legends get a soft white glow; the rest a thin solid outline that reads as a bolder print
            outlineWidth={glow ? "12%" : "3%"}
            outlineBlur={glow ? "40%" : 0}
            outlineColor={glow ? "#FFFFFF" : ink}
            outlineOpacity={glow || 1}
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

/** Studio lights, all scaled by the viewer's Brightness. The Lightformers are rendered once into a cube
 * map on the spot (no HDR download) and give the aluminium plate soft reflections. */
function Lights({ brightness: b }: { brightness: number }) {
  return (
    <>
      <hemisphereLight color="#FFFFFF" groundColor="#2A2A30" intensity={1.3 * b} />
      <directionalLight position={[6, 12, 8]} intensity={2.6 * b} />
      <directionalLight position={[-8, 6, 4]} intensity={0.8 * b} />
      {/* rim from behind so the cap edges catch a line of light */}
      <directionalLight position={[0, 5, -12]} intensity={1.4 * b} color="#DCE6FF" />
      <Environment resolution={128} environmentIntensity={b}>
        <Lightformer intensity={3} position={[0, 12, 0]} scale={[24, 12]} />
        <Lightformer intensity={1.5} position={[-14, 5, 0]} scale={[14, 4]} />
        <Lightformer intensity={1.5} position={[14, 5, 0]} scale={[14, 4]} />
      </Environment>
    </>
  );
}

type RigProps = { yaw: number; pitch: number; zoom: number; radius: number; onViewChange: Keyboard3DProps["onViewChange"] };

const near = (a: number, b: number) => Math.abs(a - b) < 1e-3;

/** OrbitControls kept in two-way sync with the view props. Lives inside <Canvas>. */
function Rig({ yaw, pitch, zoom, radius, onViewChange }: RigProps) {
  const aspect = useThree((s) => (s.size.width > 0 && s.size.height > 0 ? s.size.width / s.size.height : 1));
  const fit = fitDistance(radius, aspect);
  const ref = useRef<Controls>(null);
  const applying = useRef(false);

  // props -> camera (sliders, Flip, Reset)
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const v = clampView({ yaw, pitch, zoom });
    const dist = fit / v.zoom;
    if (near(wrapAngle(c.getAzimuthalAngle() - v.yaw), 0) && near(c.getPolarAngle(), v.pitch) && near(c.getDistance(), dist)) return;
    applying.current = true;
    c.setAzimuthalAngle(v.yaw); // three-stdlib OrbitControls (drei) has these setters; vanilla three's does not
    c.setPolarAngle(v.pitch);
    c.object.position.sub(c.target).setLength(dist).add(c.target); // three-stdlib has no setDistance
    c.update();
    applying.current = false;
  }, [yaw, pitch, zoom, fit]);

  // camera -> props (right-drag, wheel, middle-drag)
  const onChange = useCallback(() => {
    const c = ref.current;
    if (!c || applying.current) return;
    onViewChange(clampView({ yaw: c.getAzimuthalAngle(), pitch: c.getPolarAngle(), zoom: fit / c.getDistance() }));
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
      // yaw goes all the way round; tilt stops above the horizon so the underside never shows
      minPolarAngle={PITCH_MIN}
      maxPolarAngle={PITCH_MAX}
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
  pitch,
  zoom,
  lighting,
  onViewChange,
}: Keyboard3DProps) {
  const { width, depth, radius } = useMemo(() => measure(layout.keys), [layout]);
  const square = fitDistance(radius, 1);
  // initial camera only; Rig fits it to the canvas aspect and keeps it in sync afterwards
  const [initial] = useState(() => cameraPos(clampView({ yaw, pitch, zoom }), square));

  return (
    <Canvas
      dpr={[1, 2]}
      camera={{ fov: FOV, near: 0.1, far: square * 4, position: initial }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <Lights brightness={lighting.brightness} />
      {/* plate: space gray aluminium */}
      <RoundedBox args={[width + PLATE_MARGIN * 2, PLATE_T, depth + PLATE_MARGIN * 2]} radius={0.12} smoothness={6}>
        <meshStandardMaterial color="#6A6C73" metalness={0.5} roughness={0.4} />
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
          backlight={lighting.backlight}
          onSelect={onSelect}
        />
      ))}
      <Rig yaw={yaw} pitch={pitch} zoom={zoom} radius={radius} onViewChange={onViewChange} />
    </Canvas>
  );
}
