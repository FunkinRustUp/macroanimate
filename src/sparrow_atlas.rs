// src/sparrow_atlas.rs

#[derive(Clone)]
pub struct SparrowFrame {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub frame_x: f32,
    pub frame_y: f32,
}

/// Parses a raw Sparrow XML string asset slice into a collection of frames matching an animation prefix.
pub fn parse_sparrow(xml: &str, anim_prefix: &str) -> Vec<SparrowFrame> {
    let mut frames = Vec::new();

    for line in xml.lines() {
        let line = line.trim();
        if !line.starts_with("<SubTexture") {
            continue;
        }

        let get = |key: &str| -> Option<f32> {
            let pat = format!("{}=\"", key);
            let start = line.find(&pat)? + pat.len();
            let end = line[start..].find('"')? + start;
            line[start..end].parse().ok()
        };

        let Some(ns) = line.find("name=\"") else {
            continue;
        };
        let name_start = ns + 6;
        let Some(ne) = line[name_start..].find('"') else {
            continue;
        };
        let name = &line[name_start..name_start + ne];

        if !name.starts_with(anim_prefix) {
            continue;
        }

        frames.push(SparrowFrame {
            x: get("x").unwrap_or(0.0),
            y: get("y").unwrap_or(0.0),
            width: get("width").unwrap_or(0.0),
            height: get("height").unwrap_or(0.0),
            frame_x: get("frameX").unwrap_or(0.0),
            frame_y: get("frameY").unwrap_or(0.0),
        });
    }

    frames
}
