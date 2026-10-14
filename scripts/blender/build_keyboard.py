"""Build the Macally keyboard in Blender from src/lib/layout.json and render a hero image.

Run inside Blender (Blender 4.2+):
    blender --background --python scripts/blender/build_keyboard.py -- --out docs/images/keyboard.jpg
or exec() it from the Blender MCP / Python console with REPO set to the repository root.
1 key unit = 1 Blender unit. Everything lands in a new scene called "MacallyHero".
"""
import json
import math
import os
import sys

import bpy

REPO = globals().get("REPO") or os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = globals().get("OUT")
if OUT is None and "--" in sys.argv:
    args = sys.argv[sys.argv.index("--") + 1:]
    if "--out" in args:
        OUT = args[args.index("--out") + 1]
OUT = OUT or os.path.join(REPO, "docs", "images", "keyboard.jpg")

GAP = 0.1          # space between neighbouring caps
FN_GAP = 0.15      # extra air under the F row
KEY_H = 0.22       # keycap height
PLATE_T = 0.28     # plate thickness
MARGIN = 0.45      # plate border around the keys
GRAY = (0.32, 0.33, 0.35, 1)   # space gray (linear)
CAP = (0.012, 0.012, 0.014, 1)  # black keycap
LED = (1.0, 1.0, 1.0, 1)        # white backlight


def row_top(row):
    return 0.0 if row == 0 else 0.6 + FN_GAP + (row - 1)


def material(name, base, metallic=0.0, roughness=0.5, emission=None, strength=0.0):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    p = m.node_tree.nodes.get("Principled BSDF")
    p.inputs["Base Color"].default_value = base
    p.inputs["Metallic"].default_value = metallic
    p.inputs["Roughness"].default_value = roughness
    if emission:
        p.inputs["Emission Color"].default_value = emission
        p.inputs["Emission Strength"].default_value = strength
    return m


def box(name, size, loc, mat, bevel, coll):
    mesh = bpy.data.meshes.new(name)
    obj = bpy.data.objects.new(name, mesh)
    coll.objects.link(obj)
    sx, sy, sz = (s / 2 for s in size)
    verts = [(x * sx, y * sy, z * sz) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    faces = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj.location = loc
    obj.data.materials.append(mat)
    mod = obj.modifiers.new("Bevel", "BEVEL")
    mod.width = bevel
    mod.segments = 4
    mod.limit_method = "NONE"
    for poly in mesh.polygons:
        poly.use_smooth = True
    return obj


def build():
    layout = json.load(open(os.path.join(REPO, "src", "lib", "layout.json"), encoding="utf-8"))
    keys = layout["keys"]
    width = max(k["x"] + k["w"] for k in keys)
    depth = max(row_top(k["row"]) + k["h"] for k in keys)

    scene = bpy.data.scenes.get("MacallyHero") or bpy.data.scenes.new("MacallyHero")
    for o in list(scene.collection.all_objects):
        bpy.data.objects.remove(o, do_unlink=True)
    coll = scene.collection
    if bpy.context.window:
        bpy.context.window.scene = scene

    plate_mat = material("Plate", GRAY, metallic=0.85, roughness=0.32)
    cap_mat = material("Keycap", CAP, roughness=0.55)
    led_mat = material("Legend", LED, emission=LED, strength=6.0)
    glow_mat = material("Backlight", (0, 0, 0, 1), emission=LED, strength=2.2)
    floor_mat = material("Floor", (0.012, 0.012, 0.014, 1), roughness=0.85)

    box("Plate", (width + MARGIN * 2, depth + MARGIN * 2, PLATE_T), (0, 0, PLATE_T / 2), plate_mat, 0.12, coll)
    # light that bleeds between the caps, like the real white backlight
    for k in keys:
        cx = k["x"] + k["w"] / 2 - width / 2
        cy = -(row_top(k["row"]) + k["h"] / 2 - depth / 2)
        box("Glow_" + k["code"], (k["w"] - GAP + 0.04, k["h"] - GAP + 0.04, 0.01),
            (cx, cy, PLATE_T + 0.005), glow_mat, 0.003, coll)

    font_path = os.path.join(REPO, "src", "assets", "fonts", "JetBrainsMono-Variable.ttf")
    font = bpy.data.fonts.load(font_path, check_existing=True)
    for k in keys:
        cx = k["x"] + k["w"] / 2 - width / 2
        cy = -(row_top(k["row"]) + k["h"] / 2 - depth / 2)
        zc = PLATE_T + 0.03 + KEY_H / 2
        box("Key_" + k["code"], (k["w"] - GAP, k["h"] - GAP, KEY_H), (cx, cy, zc), cap_mat, 0.05, coll)
        legend = k["mac"]
        if not legend:
            continue
        curve = bpy.data.curves.new("Legend_" + k["code"], "FONT")
        curve.body = legend
        curve.font = font
        curve.align_x = "CENTER"
        curve.align_y = "CENTER"
        short = len(legend) <= 2
        curve.size = (0.2 if k["h"] < 1 else 0.3) if short else (0.11 if k["h"] < 1 else 0.14)
        t = bpy.data.objects.new("Legend_" + k["code"], curve)
        coll.objects.link(t)
        t.location = (cx, cy, zc + KEY_H / 2 + 0.002)
        t.data.materials.append(led_mat)

    # logo between the arrows and the nav cluster, like the real board
    logo = bpy.data.curves.new("Logo", "FONT")
    logo.body = "MACALLY"
    logo.font = font
    logo.size = 0.2
    logo.space_character = 1.3
    logo.align_x = "CENTER"
    lo = bpy.data.objects.new("Logo", logo)
    coll.objects.link(lo)
    lo.location = (16.95 - width / 2, -(row_top(3) + 0.5 - depth / 2), PLATE_T + 0.002)
    lo.data.materials.append(material("LogoInk", (0.03, 0.03, 0.035, 1), roughness=0.6))

    floor = box("Floor", (80, 80, 0.02), (0, 0, -0.01), floor_mat, 0.0, coll)
    floor.modifiers.clear()

    # camera: front-left, elevated, like the product shots
    cam_data = bpy.data.cameras.new("Camera")
    cam_data.lens = 50
    cam = bpy.data.objects.new("Camera", cam_data)
    coll.objects.link(cam)
    cam.location = (-13.5, -30.5, 17.5)
    target = bpy.data.objects.new("Target", None)
    coll.objects.link(target)
    target.location = (0.2, 0.6, 0)
    con = cam.constraints.new("TRACK_TO")
    con.target = target
    con.track_axis = "TRACK_NEGATIVE_Z"
    con.up_axis = "UP_Y"
    scene.camera = cam

    def area(name, loc, energy, size, color=(1, 1, 1)):
        d = bpy.data.lights.new(name, "AREA")
        d.energy = energy
        d.size = size
        d.color = color
        o = bpy.data.objects.new(name, d)
        coll.objects.link(o)
        o.location = loc
        c = o.constraints.new("TRACK_TO")
        c.target = target
        c.track_axis = "TRACK_NEGATIVE_Z"
        c.up_axis = "UP_Y"

    area("Key", (-8, -6, 14), 2500, 10)
    area("Rim", (10, 14, 5), 900, 8, (0.8, 0.86, 1.0))
    area("Fill", (12, -10, 4), 400, 6)

    world = bpy.data.worlds.get("HeroWorld") or bpy.data.worlds.new("HeroWorld")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.006, 0.006, 0.007, 1)
    scene.world = world

    engines = {e.identifier for e in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items}
    scene.render.engine = "BLENDER_EEVEE" if "BLENDER_EEVEE" in engines else "BLENDER_EEVEE_NEXT"
    scene.render.resolution_x = 1920
    scene.render.resolution_y = 900
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = "AgX"
    scene.view_settings.look = "AgX - Medium High Contrast"
    scene.render.image_settings.file_format = "JPEG"
    scene.render.image_settings.quality = 90
    try:
        scene.eevee.use_raytracing = True
        scene.eevee.taa_render_samples = 64
    except AttributeError:
        pass
    return scene


scene = build()
os.makedirs(os.path.dirname(OUT), exist_ok=True)
scene.render.filepath = OUT
bpy.ops.render.render(write_still=True, scene=scene.name)
result = {"out": OUT, "engine": scene.render.engine, "objects": len(scene.collection.all_objects)}
print(result)
