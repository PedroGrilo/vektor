//! Built-in CS2 config presets. Only convars and cs2_video.txt keys known to
//! exist in CS2 are used.

use std::collections::BTreeMap;

use super::{ConfigPreset, Cs2Config, VideoSettings};

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Common low-latency convars shared by every preset.
const BASE: &[(&str, &str)] = &[
    ("fps_max_ui", "120"),
    ("engine_low_latency_sleep_after_client_tick", "true"),
];

fn preset(
    id: &str,
    name: &str,
    description: &str,
    tag: &str,
    convars: &[(&str, &str)],
    video: &[(&str, &str)],
) -> ConfigPreset {
    let mut c = map(BASE);
    c.extend(map(convars));
    ConfigPreset {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        tag: tag.into(),
        config: Cs2Config {
            convars: c,
            binds: BTreeMap::new(),
            video: VideoSettings { values: map(video), ..Default::default() },
            reload_key: None,
        },
    }
}

/// Lowest-cost video settings (everything off / low).
const VIDEO_LOW: &[(&str, &str)] = &[
    ("setting.msaa_samples", "0"),
    ("setting.r_csgo_cmaa_enable", "0"),
    ("setting.videocfg_shadow_quality", "0"),
    ("setting.videocfg_dynamic_shadows", "0"),
    ("setting.videocfg_texture_detail", "0"),
    ("setting.r_texturefilteringquality", "0"),
    ("setting.shaderquality", "0"),
    ("setting.videocfg_particle_detail", "0"),
    ("setting.videocfg_ao_detail", "0"),
    ("setting.videocfg_hdr_detail", "3"),
    ("setting.videocfg_fsr_detail", "0"),
    ("setting.r_low_latency", "1"),
    ("setting.mat_vsync", "0"),
    ("setting.fullscreen", "1"),
];

pub fn all() -> Vec<ConfigPreset> {
    vec![
        preset(
            "max-fps",
            "FPS Máximo",
            "Tudo no mínimo e sem limite de FPS. Ideal para monitores de alta frequência.",
            "fps",
            &[("fps_max", "0")],
            VIDEO_LOW,
        ),
        preset(
            "competitive",
            "Competitivo",
            "Equilíbrio entre FPS e clareza: sombras dinâmicas ligadas para ver adversários, \
             restantes efeitos em baixo, radar ampliado e centrado.",
            "competitive",
            &[
                ("fps_max", "0"),
                ("cl_radar_always_centered", "0"),
                ("cl_radar_scale", "0.4"),
                ("cl_hud_radar_scale", "1.15"),
                ("viewmodel_fov", "68"),
                ("viewmodel_offset_x", "2.5"),
                ("viewmodel_offset_y", "0"),
                ("viewmodel_offset_z", "-1.5"),
            ],
            &[
                ("setting.msaa_samples", "4"),
                ("setting.r_csgo_cmaa_enable", "0"),
                ("setting.videocfg_shadow_quality", "1"),
                ("setting.videocfg_dynamic_shadows", "1"),
                ("setting.videocfg_texture_detail", "1"),
                ("setting.r_texturefilteringquality", "1"),
                ("setting.shaderquality", "0"),
                ("setting.videocfg_particle_detail", "0"),
                ("setting.videocfg_ao_detail", "0"),
                ("setting.videocfg_hdr_detail", "3"),
                ("setting.videocfg_fsr_detail", "0"),
                ("setting.r_low_latency", "1"),
                ("setting.mat_vsync", "0"),
                ("setting.fullscreen", "1"),
            ],
        ),
        preset(
            "visibility",
            "Visibilidade",
            "Mira estática de alto contraste e gamma mais clara para destacar modelos em zonas escuras.",
            "visibility",
            &[
                ("fps_max", "0"),
                ("r_fullscreen_gamma", "2.2"),
                ("cl_crosshairstyle", "4"),
                ("cl_crosshairsize", "2"),
                ("cl_crosshairthickness", "0.5"),
                ("cl_crosshairgap", "-2"),
                ("cl_crosshair_drawoutline", "1"),
                ("cl_crosshair_outlinethickness", "1"),
                ("cl_crosshaircolor", "5"),
                ("cl_crosshaircolor_r", "0"),
                ("cl_crosshaircolor_g", "255"),
                ("cl_crosshaircolor_b", "255"),
                ("cl_crosshairdot", "0"),
                ("cl_crosshair_t", "0"),
            ],
            &[
                ("setting.msaa_samples", "4"),
                ("setting.videocfg_shadow_quality", "1"),
                ("setting.videocfg_dynamic_shadows", "1"),
                ("setting.videocfg_texture_detail", "1"),
                ("setting.r_texturefilteringquality", "3"),
                ("setting.videocfg_hdr_detail", "3"),
                ("setting.r_low_latency", "1"),
                ("setting.mat_vsync", "0"),
            ],
        ),
        preset(
            "low-end",
            "PC Fraco",
            "Para portáteis e PCs antigos: tudo no mínimo, FSR em desempenho e FPS limitado para \
             tempos de frame estáveis.",
            "lowend",
            &[("fps_max", "144")],
            &[
                ("setting.msaa_samples", "0"),
                ("setting.r_csgo_cmaa_enable", "0"),
                ("setting.videocfg_shadow_quality", "0"),
                ("setting.videocfg_dynamic_shadows", "0"),
                ("setting.videocfg_texture_detail", "0"),
                ("setting.r_texturefilteringquality", "0"),
                ("setting.shaderquality", "0"),
                ("setting.videocfg_particle_detail", "0"),
                ("setting.videocfg_ao_detail", "0"),
                ("setting.videocfg_hdr_detail", "3"),
                ("setting.videocfg_fsr_detail", "4"),
                ("setting.r_low_latency", "1"),
                ("setting.mat_vsync", "0"),
                ("setting.fullscreen", "1"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_valid() {
        let p = all();
        assert_eq!(p.len(), 4);
        for pr in &p {
            super::super::config::validate(&pr.config).unwrap();
        }
        let ids: Vec<_> = p.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, ["max-fps", "competitive", "visibility", "low-end"]);
    }
}
