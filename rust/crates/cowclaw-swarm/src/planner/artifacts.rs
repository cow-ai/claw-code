use std::path::Path;
use crate::planner::profile::ProfileId;
use crate::planner::xml_plan::XmlPlan;

#[derive(Debug, Clone)]
pub struct PhaseNode {
    pub id: String,
    pub profile: ProfileId,
    pub waves: Vec<WaveNode>,
}

#[derive(Debug, Clone)]
pub struct WaveNode {
    pub id: String,
    pub plans: Vec<XmlPlan>,
}

pub struct PlanningTree;

impl PlanningTree {
    pub fn write(root: &Path, phase: &PhaseNode) -> crate::Result<()> {
        let phase_dir = root.join(&phase.id);
        std::fs::create_dir_all(&phase_dir)?;
        // Write PHASE.md with profile frontmatter
        let phase_md = format!(
            "---\nid: {}\nprofile: {:?}\n---\n# Phase {}\n",
            phase.id, phase.profile, phase.id
        );
        std::fs::write(phase_dir.join("PHASE.md"), phase_md)?;

        for wave in &phase.waves {
            let wave_dir = root.join(&wave.id);
            std::fs::create_dir_all(&wave_dir)?;
            let wave_md = format!("---\nid: {}\n---\n# Wave {}\n", wave.id, wave.id);
            std::fs::write(wave_dir.join("WAVE.md"), wave_md)?;

            for plan in &wave.plans {
                // plan.id is "ph1/w1/plan-01" → plan dir is last segment
                let plan_slug = plan.id.split('/').next_back().unwrap_or("plan");
                let plan_dir = wave_dir.join(plan_slug);
                std::fs::create_dir_all(&plan_dir)?;
                let xml = plan.to_xml().map_err(crate::Error::Other)?;
                std::fs::write(plan_dir.join("PLAN.xml"), xml)?;
            }
        }
        Ok(())
    }

    pub fn load(root: &Path) -> crate::Result<Vec<PhaseNode>> {
        let mut phases = Vec::new();
        if !root.exists() { return Ok(phases); }

        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() { continue; }
            let phase_md = path.join("PHASE.md");
            if !phase_md.exists() { continue; }

            let content = std::fs::read_to_string(&phase_md)?;
            let id = path.file_name().unwrap().to_string_lossy().to_string();
            let profile = extract_profile_from_frontmatter(&content);

            let mut waves = Vec::new();
            for wave_entry in std::fs::read_dir(&path)? {
                let wave_entry = wave_entry?;
                let wave_path = wave_entry.path();
                if !wave_path.is_dir() { continue; }
                let wave_md = wave_path.join("WAVE.md");
                if !wave_md.exists() { continue; }

                let full_wave_id = format!("{}/{}", id, wave_path.file_name().unwrap().to_string_lossy());
                let mut plans = Vec::new();

                for plan_entry in std::fs::read_dir(&wave_path)? {
                    let plan_entry = plan_entry?;
                    let plan_path = plan_entry.path();
                    if !plan_path.is_dir() { continue; }
                    let plan_xml = plan_path.join("PLAN.xml");
                    if !plan_xml.exists() { continue; }
                    let xml_content = std::fs::read_to_string(&plan_xml)?;
                    if let Ok(plan) = XmlPlan::from_xml(&xml_content) {
                        plans.push(plan);
                    }
                }
                if !plans.is_empty() {
                    waves.push(WaveNode { id: full_wave_id, plans });
                }
            }
            phases.push(PhaseNode { id, profile, waves });
        }
        Ok(phases)
    }
}

fn extract_profile_from_frontmatter(content: &str) -> ProfileId {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            for line in rest[..end].lines() {
                if let Some(v) = line.strip_prefix("profile: ") {
                    return match v.trim() {
                        "P1" => ProfileId::P1, "P2" => ProfileId::P2,
                        "P3" => ProfileId::P3,
                        "P5" => ProfileId::P5, "P6" => ProfileId::P6,
                        "P7" => ProfileId::P7, "P8" => ProfileId::P8,
                        "P9" => ProfileId::P9,
                        _ => ProfileId::P4,
                    };
                }
            }
        }
    }
    ProfileId::P4
}
