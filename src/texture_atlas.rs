// src/animate_atlas.rs
use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::{Texture2D, vec2, vec3, vec4};
use std::collections::HashMap;

pub struct DrawPart {
    pub sprite_name: String,
    pub matrix: [f32; 6],
}

pub struct AnimateSprite {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub rotated: bool,
}

pub struct AnimateAtlas {
    pub sprites: HashMap<String, AnimateSprite>,
    pub symbols: HashMap<String, serde_json::Value>,
    pub animations: HashMap<String, (usize, usize, String, [f32; 6])>,
    pub sheet_w: f32,
    pub sheet_h: f32,
    pub canvas_w: f32,
    pub canvas_h: f32,
}

/// Multiply two Flash/Animate 2D affine matrices.
///
/// Flash column-major layout:
///   | a  c  tx |
///   | b  d  ty |
///   | 0  0   1 |
///
/// Composition: result = parent * child
pub fn mul_matrix(parent: &[f32; 6], child: &[f32; 6]) -> [f32; 6] {
    let [pa, pb, pc, pd, ptx, pty] = *parent;
    let [ca, cb, cc, cd, ctx, cty] = *child;
    [
        pa * ca + pc * cb,         // a
        pb * ca + pd * cb,         // b
        pa * cc + pc * cd,         // c
        pb * cc + pd * cd,         // d
        pa * ctx + pc * cty + ptx, // tx
        pb * ctx + pd * cty + pty, // ty
    ]
}

/// Identity matrix — no transform.
pub fn identity_matrix() -> [f32; 6] {
    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
}

fn json_mx(v: &serde_json::Value) -> [f32; 6] {
    let a = v.as_array().unwrap();
    std::array::from_fn(|i| a[i].as_f64().unwrap() as f32)
}

/// Return the total number of frames in a symbol's timeline
/// (max of start + duration across all layers keyframes).
fn symbol_frame_count(sym: &serde_json::Value) -> usize {
    sym["TL"]["L"]
        .as_array()
        .map(|layers| {
            layers.iter().fold(0usize, |acc, layer| {
                let layer_max = layer["FR"]
                    .as_array()
                    .map(|frames| {
                        frames.iter().fold(0usize, |a, fr| {
                            let start = fr["I"].as_u64().unwrap_or(0) as usize;
                            let dur = fr["DU"].as_u64().unwrap_or(0) as usize;
                            a.max(start + dur)
                        })
                    })
                    .unwrap_or(0);
                acc.max(layer_max)
            })
        })
        .unwrap_or(1)
        .max(1)
}

pub fn parse_animate_atlas(spritemap_json: &str, animation_json: &str) -> AnimateAtlas {
    let sm: serde_json::Value = serde_json::from_str(spritemap_json).unwrap();
    let an: serde_json::Value = serde_json::from_str(animation_json).unwrap();

    let sheet_w = sm["meta"]["size"]["w"].as_f64().unwrap_or(2044.0) as f32;
    let sheet_h = sm["meta"]["size"]["h"].as_f64().unwrap_or(2044.0) as f32;

    let canvas_w = an["MD"]["W"].as_f64().unwrap_or(1920.0) as f32;
    let canvas_h = an["MD"]["H"].as_f64().unwrap_or(1080.0) as f32;

    let sprites: HashMap<String, AnimateSprite> = sm["ATLAS"]["SPRITES"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let s = &e["SPRITE"];
            (
                s["name"].as_str().unwrap().to_owned(),
                AnimateSprite {
                    x: s["x"].as_f64().unwrap() as f32,
                    y: s["y"].as_f64().unwrap() as f32,
                    w: s["w"].as_f64().unwrap() as f32,
                    h: s["h"].as_f64().unwrap() as f32,
                    rotated: s["rotated"].as_bool().unwrap_or(false), // <-- add this
                },
            )
        })
        .collect();

    let symbols: HashMap<String, serde_json::Value> = an["SD"]["S"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["SN"].as_str().unwrap().to_owned(), s.clone()))
        .collect();

    let mut animations = HashMap::new();

    if let Some(layers) = an["AN"]["TL"]["L"].as_array() {
        if !layers.is_empty() {
            let layer0_frames = layers[0]["FR"].as_array().unwrap();

            for l0_fr in layer0_frames {
                let name = l0_fr["N"].as_str().unwrap_or("").trim().to_owned();
                if name.is_empty() {
                    continue;
                }

                let start_index = l0_fr["I"].as_u64().unwrap() as usize;
                let duration = l0_fr["DU"].as_u64().unwrap() as usize;

                let mut found_si = None;
                for layer in layers.iter().skip(1) {
                    if let Some(layer_frames) = layer["FR"].as_array() {
                        let matching_fr = layer_frames.iter().find(|l_fr| {
                            let l_start = l_fr["I"].as_u64().unwrap() as usize;
                            let l_dur = l_fr["DU"].as_u64().unwrap() as usize;
                            start_index >= l_start && start_index < l_start + l_dur
                        });

                        if let Some(fr) = matching_fr {
                            if let Some(elements) = fr["E"].as_array() {
                                if let Some(si) = elements.get(0).and_then(|e| e.get("SI")) {
                                    found_si = Some(si.clone());
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(si) = found_si {
                    let sym_name = si["SN"].as_str().unwrap().to_owned();
                    let sym_mx = json_mx(&si["MX"]);
                    animations.insert(name, (start_index, duration, sym_name, sym_mx));
                }
            }
        }
    }

    AnimateAtlas {
        sprites,
        symbols,
        animations,
        sheet_w,
        sheet_h,
        canvas_w,
        canvas_h,
    }
}

pub fn get_animate_parts(atlas: &AnimateAtlas, anim_name: &str, frame: usize) -> Vec<DrawPart> {
    let Some((_, dur, sym_name, sym_mx)) = atlas.animations.get(anim_name) else {
        return vec![];
    };
    let local_frame = frame % dur;
    let mut parts = Vec::new();
    collect_draw_parts(sym_name, local_frame, *sym_mx, &atlas.symbols, &mut parts);
    parts
}

fn collect_draw_parts(
    sym_name: &str,
    cur_frame: usize,
    parent_mx: [f32; 6],
    symbols: &HashMap<String, serde_json::Value>,
    out: &mut Vec<DrawPart>,
) {
    let Some(sym) = symbols.get(sym_name) else {
        return;
    };

    let layers = sym["TL"]["L"].as_array().unwrap();

    // Reverse matches FlxAnimate's `layers[layers.length - 1 - i]` traversal.
    for layer in layers.iter().rev() {
        let frames = layer["FR"].as_array().unwrap();

        let active_frame = frames.iter().find(|fr| {
            let start = fr["I"].as_u64().unwrap() as usize;
            let dur = fr["DU"].as_u64().unwrap() as usize;
            cur_frame >= start && cur_frame < start + dur
        });

        let Some(fr) = active_frame else {
            continue;
        };

        let frame_start = fr["I"].as_u64().unwrap() as usize;
        let local_frame = cur_frame - frame_start;

        for e in fr["E"].as_array().unwrap_or(&vec![]) {
            if let Some(asi) = e.get("ASI") {
                let local_mx = json_mx(&asi["MX"]);
                let combined = mul_matrix(&parent_mx, &local_mx);
                out.push(DrawPart {
                    sprite_name: asi["N"].as_str().unwrap().to_owned(),
                    matrix: combined,
                });
            } else if let Some(si) = e.get("SI") {
                let child_name = si["SN"].as_str().unwrap();
                let child_mx = json_mx(&si["MX"]);
                let combined = mul_matrix(&parent_mx, &child_mx);

                let loop_type = si["LP"].as_str().unwrap_or("LP");
                let first_frame = si["FF"].as_u64().unwrap_or(0) as usize;

                let child_total = symbols
                    .get(child_name)
                    .map(|s| symbol_frame_count(s))
                    .unwrap_or(1)
                    .max(1);

                let raw_ff = local_frame + first_frame;
                let child_frame = match loop_type {
                    "SF" => first_frame.min(child_total - 1), // SingleFrame
                    "PO" => raw_ff.min(child_total - 1),      // PlayOnce
                    _ => raw_ff % child_total,                // Loop (default)
                };

                collect_draw_parts(child_name, child_frame, combined, symbols, out);
            }
        }
    }
}

pub fn draw_part_mesh(
    texture: &Texture2D,
    sprite: &AnimateSprite,
    matrix: &[f32; 6],
    sheet_w: f32,
    sheet_h: f32,
    origin_x: f32,
    origin_y: f32,
) {
    let [a, b, c, d, tx, ty] = *matrix;

    let (quad_w, quad_h) = if sprite.rotated {
        (sprite.h, sprite.w)
    } else {
        (sprite.w, sprite.h)
    };

    let transform = |lx: f32, ly: f32| {
        vec3(
            origin_x + a * lx + c * ly + tx,
            origin_y + b * lx + d * ly + ty,
            0.0,
        )
    };

    let (uv_tl, uv_tr, uv_br, uv_bl) = if sprite.rotated {
        (
            vec2((sprite.x + sprite.w) / sheet_w, sprite.y / sheet_h),
            vec2(
                (sprite.x + sprite.w) / sheet_w,
                (sprite.y + sprite.h) / sheet_h,
            ),
            vec2(sprite.x / sheet_w, (sprite.y + sprite.h) / sheet_h),
            vec2(sprite.x / sheet_w, sprite.y / sheet_h),
        )
    } else {
        (
            vec2(sprite.x / sheet_w, sprite.y / sheet_h),
            vec2((sprite.x + sprite.w) / sheet_w, sprite.y / sheet_h),
            vec2(
                (sprite.x + sprite.w) / sheet_w,
                (sprite.y + sprite.h) / sheet_h,
            ),
            vec2(sprite.x / sheet_w, (sprite.y + sprite.h) / sheet_h),
        )
    };

    let vertices = vec![
        Vertex {
            position: transform(0.0, 0.0),
            uv: uv_tl,
            color: [255, 255, 255, 255],
            normal: vec4(0.0, 0.0, 1.0, 0.0),
        },
        Vertex {
            position: transform(quad_w, 0.0),
            uv: uv_tr,
            color: [255, 255, 255, 255],
            normal: vec4(0.0, 0.0, 1.0, 0.0),
        },
        Vertex {
            position: transform(quad_w, quad_h),
            uv: uv_br,
            color: [255, 255, 255, 255],
            normal: vec4(0.0, 0.0, 1.0, 0.0),
        },
        Vertex {
            position: transform(0.0, quad_h),
            uv: uv_bl,
            color: [255, 255, 255, 255],
            normal: vec4(0.0, 0.0, 1.0, 0.0),
        },
    ];

    draw_mesh(&Mesh {
        vertices,
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: Some(texture.clone()),
    });
}
