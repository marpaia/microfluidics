// The room the device is lit by: dark, with a few soft lamps overhead and about it. Clear things
// show what they reflect, so a lamp with soft sides is seen in glass as a sheen that moves across
// it as the eye does, where an evenly bright room is seen as grey.

import * as THREE from "three";

/** A lamp: a sheet that is bright at its middle and fades to nothing at its sides. */
function lamp(width: number, height: number, strength: number, tint: number): THREE.Mesh {
  const sheet = new THREE.PlaneGeometry(width, height, 16, 16);
  const places = sheet.getAttribute("position");
  const tones = new Float32Array(places.count * 3);
  const colour = new THREE.Color(tint);
  for (let at = 0; at < places.count; at += 1) {
    const [across, along] = [(2 * places.getX(at)) / width, (2 * places.getY(at)) / height];
    const fall = Math.max(0, 1 - across * across) ** 2 * Math.max(0, 1 - along * along) ** 2;
    tones.set([colour.r * strength * fall, colour.g * strength * fall, colour.b * strength * fall], at * 3);
  }
  sheet.setAttribute("color", new THREE.BufferAttribute(tones, 3));
  return new THREE.Mesh(sheet, new THREE.MeshBasicMaterial({ vertexColors: true, side: THREE.DoubleSide, blending: THREE.AdditiveBlending, depthWrite: false }));
}

/** A lamp in the direction (`turn` about the upright, `rise` above the level), facing the device. */
function hung(room: THREE.Scene, turn: number, rise: number, roll: number, width: number, height: number, strength: number, tint = 0xffffff): void {
  const held = lamp(width, height, strength, tint);
  held.position.set(Math.cos(rise) * Math.cos(turn), Math.cos(rise) * Math.sin(turn), Math.sin(rise)).multiplyScalar(30);
  held.up.set(0, 0, 1);
  held.lookAt(0, 0, 0);
  held.rotateZ(roll);
  room.add(held);
}

/** The room, about the origin, with z upward as the device has it. */
export function studio(): THREE.Scene {
  const room = new THREE.Scene();
  room.add(new THREE.Mesh(new THREE.SphereGeometry(60, 32, 16), new THREE.MeshBasicMaterial({ color: 0x0b0d10, side: THREE.BackSide })));
  // Overhead, a long narrow lamp a little off the upright: from straight above it is a streak
  // across the glass.
  hung(room, 0.6, 1.42, 0.7, 60, 7, 5);
  // About the device, lamps with dark between them, so that turning it carries a sheen across.
  hung(room, 0.9, 0.62, 0.25, 24, 7, 5);
  hung(room, 2.7, 0.5, -0.3, 22, 6, 4, 0xe8f2ff);
  hung(room, -1.9, 0.7, 0.2, 20, 7, 4);
  hung(room, -0.4, 0.35, 0, 30, 5, 3, 0xe8f2ff);
  // Level with the device, a faint band all about.
  for (let turn = 0; turn < 6; turn += 1) hung(room, turn * (Math.PI / 3) + 0.2, 0.05, 0, 36, 8, 0.9, 0xdfeaf5);
  // And a fainter one below it, which the sides throw back to an eye above them.
  for (let turn = 0; turn < 6; turn += 1) hung(room, turn * (Math.PI / 3) + 0.7, -0.4, 0, 36, 14, 0.5, 0xdfeaf5);
  // Beneath, one lamp, for the eye that looks up through the glass.
  hung(room, 1.2, -1.2, 0.5, 44, 10, 3);
  return room;
}
