// src/animate_atlas.rs
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::{vec2, vec3, vec4, Texture2D};
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
}

pub struct AnimateAtlas {
    pub sprites: HashMap<String, AnimateSprite>,
    pub symbols: HashMap<String, serde_json::Value>,
    pub animations: HashMap<String, (usize, usize, String, [f32; 6])>,
}

pub fn mul_matrix(a: &[f32; 6], b: &[f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

fn json_mx(v: &serde_json::Value) -> [f32; 6] {
    let a = v.as_array().unwrap();
    std::array::from_fn(|i| a[i].as_f64().unwrap() as f32)
}

pub fn parse_animate_atlas(spritemap_json: &str, animation_json: &str) -> AnimateAtlas {
    let sm: serde_json::Value = serde_json::from_str(spritemap_json).unwrap();
    let an: serde_json::Value = serde_json::from_str(animation_json).unwrap();

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

    let layer0 = &an["AN"]["TL"]["L"][0]["FR"];
    let layer1 = &an["AN"]["TL"]["L"][1]["FR"];

    let mut animations = HashMap::new();
    for (l0, l1) in layer0
        .as_array()
        .unwrap()
        .iter()
        .zip(layer1.as_array().unwrap().iter())
    {
        let name = l0["N"].as_str().unwrap_or("").trim().to_owned();
        let index = l0["I"].as_u64().unwrap() as usize;
        let duration = l0["DU"].as_u64().unwrap() as usize;
        let si = &l1["E"][0]["SI"];
        let sym_name = si["SN"].as_str().unwrap().to_owned();
        let sym_mx = json_mx(&si["MX"]);
        animations.insert(name, (index, duration, sym_name, sym_mx));
    }

    AnimateAtlas {
        sprites,
        symbols,
        animations,
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
    timeline_frame: usize,
    parent_mx: [f32; 6],
    symbols: &HashMap<String, serde_json::Value>,
    out: &mut Vec<DrawPart>,
) {
    let Some(sym) = symbols.get(sym_name) else {
        return;
    };
    let layers = sym["TL"]["L"].as_array().unwrap();
    for layer in layers.iter().rev() {
        let frames = layer["FR"].as_array().unwrap();
        for fr in frames {
            let start = fr["I"].as_u64().unwrap() as usize;
            let dur = fr["DU"].as_u64().unwrap() as usize;
            if timeline_frame < start || timeline_frame >= start + dur {
                continue;
            }
            for e in fr["E"].as_array().unwrap() {
                if let Some(asi) = e.get("ASI") {
                    let local_mx = json_mx(&asi["MX"]);
                    let combined = mul_matrix(&parent_mx, &local_mx);
                    out.push(DrawPart {
                        sprite_name: asi["N"].as_str().unwrap().to_owned(),
                        matrix: combined,
                    });
                } else if let Some(si) = e.get("SI") {
                    let child_name = si["SN"].as_str().unwrap();
                    let ff = si["FF"].as_u64().unwrap_or(0) as usize;
                    let child_mx = json_mx(&si["MX"]);
                    let combined = mul_matrix(&parent_mx, &child_mx);
                    collect_draw_parts(child_name, ff, combined, symbols, out);
                }
            }
            break;
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

    let transform = |lx: f32, ly: f32| {
        vec3(
            origin_x + a * lx + c * ly + tx,
            origin_y + b * lx + d * ly + ty,
            0.0,
        )
    };

    let w = sprite.w;
    let h = sprite.h;

    let vertices = vec![
        Vertex {
            position: transform(0.0, 0.0),
            uv: vec2(sprite.x / sheet_w, sprite.y / sheet_h),
            color: [255, 255, 255, 255],
            normal: vec4(0.0, 0.0, 1.0, 0.0),
        },
        Vertex {
            position: transform(w, 0.0),
            uv: vec2((sprite.x + sprite.w) / sheet_w, sprite.y / sheet_h),
            color: [255, 255, 255, 255],
            normal: vec4(0.0, 0.0, 1.0, 0.0),
        },
        Vertex {
            position: transform(w, h),
            uv: vec2(
                (sprite.x + sprite.w) / sheet_w,
                (sprite.y + sprite.h) / sheet_h,
            ),
            color: [255, 255, 255, 255],
            normal: vec4(0.0, 0.0, 1.0, 0.0),
        },
        Vertex {
            position: transform(0.0, h),
            uv: vec2(sprite.x / sheet_w, (sprite.y + sprite.h) / sheet_h),
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
