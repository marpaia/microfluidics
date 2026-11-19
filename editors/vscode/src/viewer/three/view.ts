// The device drawn in three dimensions: glass, a slab of clear rubber, channels of dyed liquid at
// their depths, and a hole at each access point, seen from an eye that can be moved about it. It
// draws only when asked, and what is asked of it is the viewer's state.

import * as THREE from "three";

import type { Scene } from "../../scene";
import type { Theme } from "../canvas";
import { type Rect, type Shown, at, within } from "../geometry";
import { painter } from "../paint";
import type { State } from "../state";
import { type Eye, type Look, type Vector, FIELD, LOOKS, axes, framed, neared, onLevel, orbited, panned, pixelsPerMm, position, projected, sight } from "./camera";
import { bodies, glassRect, holeRadius, solids } from "./device";
import { studio } from "./studio";

const MM = 1e-3;

/** A box. Given two materials, the first is its upper and lower faces and the second its sides. */
function box(min: Vector, max: Vector, material: THREE.Material | [THREE.Material, THREE.Material]): THREE.Mesh {
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(max[0] - min[0], max[1] - min[1], max[2] - min[2]), Array.isArray(material) ? [material[1], material[1], material[1], material[1], material[0], material[0]] : material);
  mesh.position.set((min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2);
  return mesh;
}

/** The edges of a box, drawn faintly so that clear things keep their outline. */
function edges(mesh: THREE.Mesh, colour: number, strength: number): THREE.LineSegments {
  const lines = new THREE.LineSegments(new THREE.EdgesGeometry(mesh.geometry), new THREE.LineBasicMaterial({ color: colour, transparent: true, opacity: strength }));
  lines.position.copy(mesh.position);
  return lines;
}

function disposed(held: THREE.Object3D): void {
  held.traverse((part) => {
    const mesh = part as THREE.Mesh;
    mesh.geometry?.dispose();
    for (const material of [mesh.material].flat()) material?.dispose();
  });
}

export class Space {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly world = new THREE.Scene();
  private readonly camera = new THREE.PerspectiveCamera(FIELD, 1, 0.1, 1000);
  private readonly ray = new THREE.Raycaster();
  private readonly sun = new THREE.DirectionalLight(0xffffff, 1.6);
  /** What is made from the scene, and remade when it changes. */
  private device = new THREE.Group();
  private layers: { mesh: THREE.InstancedMesh; layer: string; paths: string[] }[] = [];
  private cones: THREE.InstancedMesh | undefined;
  private made: { scene?: Scene; hidden?: Set<string>; depth?: number; ground?: string } = {};
  private toned: { scene?: Scene; mode?: string; selection?: string; failed?: boolean } = {};
  private eye: Eye = { target: [0, 0, 0], distance: 100, ...LOOKS.tilted };
  private size: [number, number] = [1, 1];
  /** Whether what is shown is the whole device as it was framed, to be framed again when its room changes. */
  private fitted = true;
  /** The room left about the device for what lies over the stage's edges: sides, above, below. */
  private room: [number, number, number] = [24, 52, 64];
  private scene: Scene | undefined;
  private hidden = new Set<string>();

  /** `changed` is called when the eye has moved and the chip must be drawn again. */
  constructor(canvas: HTMLCanvasElement, private readonly changed: () => void) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, powerPreference: "high-performance" });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    // Colours stand for values, and the editor's ground is seen through the glass: both are
    // carried to the screen as they are, the dark ones as much as the bright.
    this.renderer.toneMapping = THREE.LinearToneMapping;
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    const room = new THREE.PMREMGenerator(this.renderer);
    this.world.environment = room.fromScene(studio(), 0.01).texture;
    room.dispose();
    this.sun.castShadow = true;
    this.sun.shadow.mapSize.set(2048, 2048);
    this.sun.shadow.bias = -0.0005;
    this.world.add(this.sun, this.sun.target, new THREE.HemisphereLight(0xffffff, 0xb8c4cc, 1.1), this.device);
  }

  resized(width: number, height: number): void {
    if (width <= 0 || height <= 0) return;
    const same = this.size[0] === width && this.size[1] === height;
    this.size = [width, height];
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    // The whole device, when that is what was shown, is framed again in its new room.
    if (!same && this.fitted && this.scene) this.frame();
  }

  /** The glass, the slab, the holes, the bench and the light, for a scene. */
  private build(scene: Scene, state: State, tones: Theme): void {
    disposed(this.device);
    this.device.clear();
    this.layers = [];
    this.cones = undefined;
    const { slab, glass } = bodies(scene);
    const ground = new THREE.Color(tones.ground);
    const light = ground.getHSL({ h: 0, s: 0, l: 0 }).l > 0.6;
    this.renderer.setClearColor(ground);

    // Clear things bend no light here: what is seen through them is where it is, so that the
    // names written over the chip lie on what they name.
    // Clear things are drawn a little behind what touches them, so that a channel a micrometre
    // deep is not lost in the face it lies on.
    const behind = { polygonOffset: true, polygonOffsetFactor: 2, polygonOffsetUnits: 2 };
    // Glass is clear through its faces and green through its sides, as a pane is seen edge on;
    // rubber is clear through its faces and a little clouded where it was cut.
    const pane = box(glass.min, glass.max, [
      new THREE.MeshPhysicalMaterial({ color: 0xf2faf7, transmission: 1, roughness: 0.02, ior: 1.52, thickness: 0, ...behind }),
      new THREE.MeshPhysicalMaterial({ color: 0x74c9a8, transmission: 0.55, roughness: 0.18, ior: 1.52, thickness: 0, ...behind }),
    ]);
    const rubber = box(slab.min, slab.max, [
      new THREE.MeshPhysicalMaterial({ color: 0xf6fafd, transmission: 1, roughness: 0.07, ior: 1.41, thickness: 0, clearcoat: 0.3, clearcoatRoughness: 0.25, ...behind }),
      new THREE.MeshPhysicalMaterial({ color: 0xe6f1fa, transmission: 1, roughness: 0.3, ior: 1.41, thickness: 0, ...behind }),
    ]);
    pane.castShadow = rubber.castShadow = true;
    this.device.add(pane, rubber, edges(pane, light ? 0x4f8f7c : 0xa8e6d0, light ? 0.55 : 0.4), edges(rubber, light ? 0x56728a : 0xe4f2fb, light ? 0.5 : 0.42));

    // The bench takes the device's shade and is otherwise the editor's ground. It is seen only
    // from above, so that from beneath nothing stands before the glass.
    const reach = Math.max(scene.glass_um[0], scene.glass_um[1]) * MM * 6;
    const bench = new THREE.Mesh(new THREE.PlaneGeometry(reach, reach), new THREE.ShadowMaterial({ opacity: light ? 0.14 : 0.5 }));
    bench.position.set((glass.min[0] + glass.max[0]) / 2, (glass.min[1] + glass.max[1]) / 2, glass.min[2] - 0.02);
    bench.receiveShadow = true;
    this.device.add(bench);
    const span = Math.max(glass.max[0] - glass.min[0], glass.max[1] - glass.min[1]);
    this.sun.position.set(bench.position.x - span * 0.45, bench.position.y - span * 0.35, span * 1.3);
    this.sun.target.position.copy(bench.position);
    const shade = this.sun.shadow.camera;
    [shade.left, shade.right, shade.bottom, shade.top, shade.near, shade.far] = [-span, span, -span, span, 0.1, span * 4];
    shade.updateProjectionMatrix();

    // A hole punched through the slab at each access point: its wall, and its rim on the upper face.
    const radius = holeRadius(scene);
    const wall = new THREE.MeshStandardMaterial({ color: light ? 0x9db2c2 : 0x7f95a6, roughness: 0.55, side: THREE.DoubleSide });
    const rim = new THREE.MeshBasicMaterial({ color: light ? 0x4b6478 : 0xf2f8fc, transparent: true, opacity: 0.8, side: THREE.DoubleSide });
    for (const [x, y] of scene.access) {
      const bore = new THREE.Mesh(new THREE.CylinderGeometry(radius, radius, slab.max[2], 40, 1, true), wall);
      bore.rotation.x = Math.PI / 2;
      bore.position.set(x * MM, y * MM, slab.max[2] / 2);
      const lip = new THREE.Mesh(new THREE.RingGeometry(radius, radius * 1.07, 48), rim);
      lip.position.set(x * MM, y * MM, slab.max[2] + 0.004);
      this.device.add(bore, lip);
    }

    // Every rectangle of a layer is one more of the same box, placed and sized.
    const unit = new THREE.BoxGeometry(1, 1, 1);
    const placing = new THREE.Matrix4();
    for (const [layer, held] of solids(scene, state.hidden, state.depth)) {
      const mesh = new THREE.InstancedMesh(unit, new THREE.MeshLambertMaterial({ emissive: 0x000000 }), held.length);
      held.forEach((solid, index) => {
        placing.makeScale(solid.max[0] - solid.min[0], solid.max[1] - solid.min[1], Math.max(solid.max[2] - solid.min[2], 1e-5));
        placing.setPosition((solid.min[0] + solid.max[0]) / 2, (solid.min[1] + solid.max[1]) / 2, (solid.min[2] + solid.max[2]) / 2);
        mesh.setMatrixAt(index, placing);
      });
      mesh.instanceMatrix.needsUpdate = true;
      mesh.computeBoundingSphere();
      this.device.add(mesh);
      this.layers.push({ mesh, layer, paths: held.map((solid) => solid.path) });
    }
  }

  /** Each rectangle's colour, and an arrowhead at each channel's downstream end when what is
   * shown is something that flows. */
  private tone(scene: Scene, state: State): void {
    const paint = painter(scene, state);
    const [colour, faint] = [new THREE.Color(), new THREE.Color(0x8696a3)];
    for (const { mesh, layer, paths } of this.layers) {
      paths.forEach((path, index) => {
        const [fill, strength] = paint(path, layer);
        mesh.setColorAt(index, colour.set(fill).lerp(faint, 1 - Math.min(strength / 0.92, 1)));
      });
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }
    if (this.cones) {
      this.device.remove(this.cones);
      disposed(this.cones);
      this.cones = undefined;
    }
    if (state.mode === "layout" || state.failure || !state.index) return;
    const heights = new Map(scene.layers.map((held) => [held.name, held.height_um * state.depth * MM]));
    const found: THREE.Matrix4[] = [];
    const [turn, upright] = [new THREE.Quaternion(), new THREE.Vector3(0, 1, 0)];
    for (const flow of scene.flows) {
      const port = flow.flow_ul_min === 0 ? undefined : state.index.ports.get(flow.flow_ul_min > 0 ? flow.end : flow.start);
      if (!port) continue;
      // A port faces the way a channel leaves by it: outward for the edge's own, inward for another's.
      const outward = within(port.path, flow.element) ? 1 : -1;
      const along = new THREE.Vector3(port.facing[0] * outward, port.facing[1] * outward, 0);
      const size = Math.min(Math.max(port.width_um * MM * 0.9, 0.06), 0.5);
      const top = (heights.get(port.layer) ?? 0) + size * 0.35;
      turn.setFromUnitVectors(upright, along);
      found.push(new THREE.Matrix4().compose(new THREE.Vector3(port.at[0] * MM - along.x * size, port.at[1] * MM - along.y * size, top), turn, new THREE.Vector3(size, size, size)));
    }
    this.cones = new THREE.InstancedMesh(new THREE.ConeGeometry(0.45, 1.3, 16), new THREE.MeshStandardMaterial({ color: 0xffffff, roughness: 0.4 }), found.length);
    found.forEach((placing, index) => this.cones!.setMatrixAt(index, placing));
    this.device.add(this.cones);
  }

  /** Draw a state. What was made for the last one is kept wherever it still holds. */
  draw(state: State, tones: Theme): void {
    const scene = state.scene;
    if (!scene) return;
    const first = this.scene === undefined;
    [this.scene, this.hidden] = [scene, state.hidden];
    const made = this.made;
    if (made.scene !== scene || made.hidden !== state.hidden || made.depth !== state.depth || made.ground !== tones.ground) {
      const resized = made.scene === undefined || made.scene.glass_um.some((side, at) => side !== scene.glass_um[at]) || made.scene.size_um.some((side, at) => side !== scene.size_um[at]);
      this.build(scene, state, tones);
      this.made = { scene, hidden: state.hidden, depth: state.depth, ground: tones.ground };
      this.toned = {};
      if (first || resized) this.frame();
    }
    const toned = this.toned;
    if (toned.scene !== scene || toned.mode !== state.mode || toned.selection !== state.selection || toned.failed !== (state.failure !== undefined)) {
      this.tone(scene, state);
      this.toned = { scene, mode: state.mode, selection: state.selection, failed: state.failure !== undefined };
    }
    const [from, { up }] = [position(this.eye), axes(this.eye)];
    this.camera.position.set(...from);
    this.camera.up.set(...up);
    this.camera.lookAt(...this.eye.target);
    [this.camera.near, this.camera.far] = [Math.max(this.eye.distance / 200, 0.005), this.eye.distance * 20 + 500];
    this.camera.updateProjectionMatrix();
    this.renderer.render(this.world, this.camera);
  }

  private moved(eye: Eye): void {
    this.eye = eye;
    this.changed();
  }

  /** Show a rectangle of the design, or with none the whole device, from where the eye is. */
  frame(rect?: Rect): void {
    if (!this.scene) return;
    const { glass, slab } = bodies(this.scene);
    const [min, max]: [Vector, Vector] = rect ? [[rect[0] * MM, rect[1] * MM, 0], [rect[2] * MM, rect[3] * MM, 0.05]] : [glass.min, [glass.max[0], glass.max[1], slab.max[2]]];
    // Something small is shown with room about it.
    const room = rect ? Math.max(max[0] - min[0], max[1] - min[1]) * 0.3 + 0.05 : 0;
    this.fitted = rect === undefined;
    this.moved(framed(this.eye, [min[0] - room, min[1] - room, min[2]], [max[0] + room, max[1] + room, max[2]], ...this.size, ...this.room));
  }

  /** Leave room about the device for the controls over the stage, or with `bare` for none. */
  clear(bare: boolean): void {
    const room: [number, number, number] = bare ? [8, 8, 8] : [24, 52, 64];
    if (room[1] === this.room[1]) return;
    this.room = room;
    if (this.fitted && this.scene) this.frame();
  }

  look(look: Look): void {
    this.eye = { ...this.eye, ...LOOKS[look] };
    this.frame();
  }

  orbitBy(dx: number, dy: number): void {
    this.moved(orbited(this.eye, dx, dy));
  }

  panBy(dx: number, dy: number): void {
    this.fitted = false;
    this.moved(panned(this.eye, dx, dy, this.size[1]));
  }

  /** Come `factor` times nearer, about a pixel when one is given. */
  zoomBy(factor: number, px?: number, py?: number): void {
    const about = px === undefined || py === undefined ? undefined : onLevel(this.eye, px, py, ...this.size);
    this.fitted = false;
    this.moved(neared(this.eye, factor, about));
  }

  /** What is drawn under a pixel, and the point of the chip's bonded face there in micrometres. */
  under(px: number, py: number): { path: string | undefined; point: [number, number] | undefined } {
    if (!this.scene) return { path: undefined, point: undefined };
    const { from, along } = sight(this.eye, px, py, ...this.size);
    this.ray.set(new THREE.Vector3(...from), new THREE.Vector3(...along));
    const hit = this.ray.intersectObjects(this.layers.map((held) => held.mesh), false)[0];
    const level = onLevel(this.eye, px, py, ...this.size);
    const point: [number, number] | undefined = level && [level[0] / MM, level[1] / MM];
    const struck = hit && this.layers.find((held) => held.mesh === hit.object)?.paths[hit.instanceId ?? -1];
    // Thin features are picked from just beside them, on the face they lie in.
    const beside = point && at(this.scene, this.hidden, point[0], point[1], 3 / (pixelsPerMm(this.eye, this.size[1]) * MM));
    return { path: struck ?? beside, point };
  }

  /** Where rectangles of the design lie on the screen, from where the eye is. */
  shown(): Shown {
    const [eye, [width, height]] = [this.eye, this.size];
    const corners = (rect: Rect): [number, number][] | undefined => {
      const found = ([[rect[0], rect[3]], [rect[2], rect[3]], [rect[2], rect[1]], [rect[0], rect[1]]] as [number, number][]).map(([x, y]) => projected(eye, [x * MM, y * MM, 0], width, height));
      return found.every((held) => held !== undefined) ? (found as [number, number][]) : undefined;
    };
    return {
      width,
      height,
      corners,
      box(rect) {
        const found = corners(rect);
        return found && [Math.min(...found.map((held) => held[0])), Math.min(...found.map((held) => held[1])), Math.max(...found.map((held) => held[0])), Math.max(...found.map((held) => held[1]))];
      },
    };
  }

  /** How many pixels a micrometre is, when the chip is seen from straight above or below and one
   * scale holds across it; `undefined` otherwise. */
  flatScale(): number | undefined {
    const level = Math.min(this.eye.tilt, Math.PI - this.eye.tilt);
    // The scale is that of the bonded face, where the channels are.
    return level < 0.02 ? pixelsPerMm(this.eye, this.size[1], Math.abs(position(this.eye)[2])) * MM : undefined;
  }

  /** The glass, in the design's plane. */
  glass(): Rect | undefined {
    return this.scene && glassRect(this.scene);
  }
}
