//! Modelos de IA descargados: Ollama, Hugging Face, LM Studio y otros cachés.
//!
//! Todos son riesgo **Avanzado** (no se preseleccionan): son descargas grandes
//! que el usuario hizo a propósito. Se listan **por modelo** para elegir cuál borrar.

use crp_core::rules::glob_under;
use crp_core::{
    Action, Category, CleanOutcome, CleanerModule, Error, Finding, ModuleMeta, PathRuleModule,
    Result, Risk, ScanContext, Target, t, tf,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

pub fn register(e: &mut crp_core::Engine) {
    e.register(Ollama {
        meta: ModuleMeta::new("ai.ollama", Category::Ai)
            .risk(Risk::Advanced)
            .closes(&["ollama.exe", "ollama app.exe"]),
    });

    e.register(PathRuleModule::new(
        ModuleMeta::new("ai.huggingface", Category::Ai).risk(Risk::Advanced),
        huggingface_targets,
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("ai.lmstudio", Category::Ai)
            .risk(Risk::Advanced)
            .closes(&["lm studio.exe"]),
        |ctx| {
            let h = &ctx.env.home;
            let mut targets = Vec::new();
            for base in [
                h.join(r".lmstudio\models"),
                h.join(r".cache\lm-studio\models"),
            ] {
                for model in glob_under(&base, r"*\*").into_iter().filter(|p| p.is_dir()) {
                    let rel = model
                        .strip_prefix(&base)
                        .unwrap_or(&model)
                        .to_string_lossy()
                        .replace('\\', "/");
                    targets.push(
                        Target::paths(&rel, [model.clone()])
                            .label(rel.clone())
                            .detail(model.display().to_string())
                            .age_days(age_days(&model)),
                    );
                }
            }
            targets
        },
    ));

    e.register(PathRuleModule::new(
        ModuleMeta::new("ai.misc", Category::Ai).risk(Risk::Moderate),
        |ctx| {
            let h = &ctx.env.home;
            let torch = ctx
                .env
                .var_path("TORCH_HOME")
                .unwrap_or_else(|| h.join(r".cache\torch"));
            vec![
                Target::contents("torch", [torch.join("hub")]),
                Target::contents("whisper", [h.join(r".cache\whisper")]).risk(Risk::Advanced),
                Target::contents("keras", [h.join(r".keras\models")]),
            ]
        },
    ));
}

/// Días desde la última modificación de `p`.
fn age_days(p: &Path) -> Option<u64> {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs() / 86_400)
}

// ── Hugging Face ────────────────────────────────────────────────────────────

fn hf_hub_dir(ctx: &ScanContext) -> PathBuf {
    let env = &ctx.env;
    if let Some(p) = env
        .var_path("HF_HUB_CACHE")
        .or_else(|| env.var_path("HUGGINGFACE_HUB_CACHE"))
    {
        return p;
    }
    let home = env.var_path("HF_HOME").unwrap_or_else(|| {
        env.var_path("XDG_CACHE_HOME")
            .unwrap_or_else(|| env.home.join(".cache"))
            .join("huggingface")
    });
    home.join("hub")
}

fn huggingface_targets(ctx: &ScanContext) -> Vec<Target> {
    let hub = hf_hub_dir(ctx);
    let Ok(entries) = fs::read_dir(&hub) else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // models--org--name  /  datasets--org--name  /  spaces--org--name
        let mut parts = name.split("--");
        let kind = match parts.next() {
            Some("models") => t("module.ai.huggingface.kind.model"),
            Some("datasets") => t("module.ai.huggingface.kind.dataset"),
            Some("spaces") => t("module.ai.huggingface.kind.space"),
            _ => continue,
        };
        let repo = parts.collect::<Vec<_>>().join("/");
        let label = format!("{repo} ({kind})");
        let path = entry.path();
        targets.push(
            Target::paths(&name, [path.clone()])
                .label(label)
                .detail(path.display().to_string())
                .age_days(age_days(&path)),
        );
    }
    targets
}

// ── Ollama ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Layer {
    digest: String,
    #[serde(default)]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    config: Option<Layer>,
    #[serde(default)]
    layers: Vec<Layer>,
}

impl Manifest {
    fn read(path: &Path) -> Option<Self> {
        serde_json::from_slice(&fs::read(path).ok()?).ok()
    }

    fn blobs(&self) -> impl Iterator<Item = &Layer> {
        self.config.iter().chain(self.layers.iter())
    }
}

/// `sha256:abc…` → `<models>\blobs\sha256-abc…`
fn blob_path(models: &Path, digest: &str) -> PathBuf {
    models.join("blobs").join(digest.replace(':', "-"))
}

/// Nombre que usa `ollama list` a partir de `manifests\<registro>\<ns>\<modelo>\<tag>`.
fn model_name(rel: &Path) -> Option<String> {
    let parts: Vec<String> = rel
        .iter()
        .map(|c| c.to_string_lossy().into_owned())
        .collect();
    let [registry, ns, model, tag] = parts.as_slice() else {
        return None;
    };
    Some(match (registry.as_str(), ns.as_str()) {
        ("registry.ollama.ai", "library") => format!("{model}:{tag}"),
        ("registry.ollama.ai", _) => format!("{ns}/{model}:{tag}"),
        _ => format!("{registry}/{ns}/{model}:{tag}"),
    })
}

struct Ollama {
    meta: ModuleMeta,
}

impl Ollama {
    fn models_dir(ctx: &ScanContext) -> PathBuf {
        ctx.env
            .var_path("OLLAMA_MODELS")
            .unwrap_or_else(|| ctx.env.home.join(r".ollama\models"))
    }

    /// Todos los manifiestos instalados.
    fn manifests(models: &Path) -> Vec<(PathBuf, Manifest)> {
        WalkDir::new(models.join("manifests"))
            .min_depth(4)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| Manifest::read(e.path()).map(|m| (e.into_path(), m)))
            .collect()
    }

    /// Cuántos modelos usan cada blob.
    fn usage(all: &[(PathBuf, Manifest)]) -> HashMap<&str, usize> {
        let mut usage = HashMap::new();
        for (_, m) in all {
            for b in m.blobs() {
                *usage.entry(b.digest.as_str()).or_insert(0) += 1;
            }
        }
        usage
    }
}

impl CleanerModule for Ollama {
    fn meta(&self) -> &ModuleMeta {
        &self.meta
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<Finding>> {
        let models = Self::models_dir(ctx);
        let all = Self::manifests(&models);
        let usage = Self::usage(&all);
        let manifests_root = models.join("manifests");

        let mut findings = Vec::new();
        for (path, manifest) in &all {
            let rel = path.strip_prefix(&manifests_root).unwrap_or(path);
            let Some(name) = model_name(rel) else {
                continue;
            };
            let total: u64 = manifest.blobs().map(|b| b.size).sum();
            let unique: u64 = manifest
                .blobs()
                .filter(|b| usage[b.digest.as_str()] == 1)
                .map(|b| b.size)
                .sum();
            let shared = total - unique;
            let mut detail = tf("module.ai.ollama.total", &[("size", &human(total))]);
            if shared > 0 {
                detail.push_str(" · ");
                detail.push_str(&tf("module.ai.ollama.shared", &[("size", &human(shared))]));
            }
            findings.push(Finding {
                id: format!("{}:{name}", self.meta.id),
                module_id: self.meta.id.clone(),
                category: self.meta.category,
                label: name,
                detail: Some(detail),
                path: Some(path.clone()),
                bytes: unique,
                items: manifest.blobs().count() as u64,
                risk: self.meta.risk,
                requires_admin: false,
                selected: false,
                age_days: age_days(path),
                warning: None,
                action: Action::Custom {
                    key: path.to_string_lossy().into_owned(),
                },
            });
        }
        findings.sort_by_key(|f| std::cmp::Reverse(f.bytes));
        Ok(findings)
    }

    /// Borra el manifiesto y los blobs que ningún otro modelo usa (lo mismo que
    /// `ollama rm`, pero sin necesitar el servidor de Ollama en marcha).
    fn clean_custom(&self, finding: &Finding, ctx: &ScanContext) -> Result<CleanOutcome> {
        let Action::Custom { key } = &finding.action else {
            return Err(Error::Unsupported(self.meta.id.clone()));
        };
        let manifest_path = PathBuf::from(key);
        let models = Self::models_dir(ctx);
        if !manifest_path.starts_with(models.join("manifests")) {
            return Err(Error::Protected(manifest_path));
        }
        let manifest = Manifest::read(&manifest_path).ok_or_else(|| {
            Error::Other(tf(
                "module.ai.ollama.bad_manifest",
                &[("path", &manifest_path.display())],
            ))
        })?;

        let others = Self::manifests(&models);
        let still_used: std::collections::HashSet<&str> = others
            .iter()
            .filter(|(p, _)| *p != manifest_path)
            .flat_map(|(_, m)| m.blobs().map(|b| b.digest.as_str()))
            .collect();

        let mut out = CleanOutcome::default();
        fs::remove_file(&manifest_path).map_err(|e| Error::io(&manifest_path, e))?;
        out.removed_items += 1;
        for blob in manifest
            .blobs()
            .filter(|b| !still_used.contains(b.digest.as_str()))
        {
            let p = blob_path(&models, &blob.digest);
            let size = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            match fs::remove_file(&p) {
                Ok(()) => {
                    out.freed_bytes += size;
                    out.removed_items += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    out.skipped_items += 1;
                    out.errors.push(format!("{}: {e}", p.display()));
                }
            }
        }
        Ok(out)
    }
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crp_core::{CleanOptions, Engine, Env, NoProgress};

    fn write_blob(models: &Path, digest: &str, size: usize) {
        let p = blob_path(models, digest);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![0u8; size]).unwrap();
    }

    fn write_manifest(models: &Path, model: &str, layers: &[(&str, u64)]) {
        let dir = models
            .join(r"manifests\registry.ollama.ai\library")
            .join(model);
        fs::create_dir_all(&dir).unwrap();
        let layers: Vec<_> = layers
            .iter()
            .map(|(d, s)| serde_json::json!({"digest": d, "size": s}))
            .collect();
        let json = serde_json::json!({"config": null, "layers": layers});
        fs::write(dir.join("latest"), json.to_string()).unwrap();
    }

    #[test]
    fn ollama_keeps_shared_layers() {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::sandbox(dir.path());
        let models = env.home.join(r".ollama\models");
        write_blob(&models, "sha256:shared", 100);
        write_blob(&models, "sha256:a", 10);
        write_blob(&models, "sha256:b", 20);
        write_manifest(
            &models,
            "alpha",
            &[("sha256:shared", 100), ("sha256:a", 10)],
        );
        write_manifest(&models, "beta", &[("sha256:shared", 100), ("sha256:b", 20)]);

        let mut engine = Engine::new();
        register(&mut engine);
        let ctx = ScanContext::new(env, false);
        let report = engine.scan(&["ai.ollama".into()], &ctx, &NoProgress);
        assert_eq!(report.findings.len(), 2);
        let alpha = report
            .findings
            .iter()
            .find(|f| f.label == "alpha:latest")
            .unwrap();
        assert_eq!(alpha.bytes, 10, "sólo cuenta lo exclusivo");

        let res = engine.clean(
            std::slice::from_ref(alpha),
            CleanOptions::default(),
            &ctx,
            &NoProgress,
        );
        assert_eq!(res.freed_bytes(), 10);
        assert!(!blob_path(&models, "sha256:a").exists());
        assert!(
            blob_path(&models, "sha256:shared").exists(),
            "capa compartida intacta"
        );
        assert!(blob_path(&models, "sha256:b").exists());
    }

    #[test]
    fn hf_lists_each_repo() {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::sandbox(dir.path());
        let hub = env.home.join(r".cache\huggingface\hub");
        fs::create_dir_all(hub.join(r"models--meta-llama--Llama-3.2-1B\blobs")).unwrap();
        fs::write(
            hub.join(r"models--meta-llama--Llama-3.2-1B\blobs\x"),
            [0u8; 5],
        )
        .unwrap();
        fs::create_dir_all(hub.join(".locks")).unwrap();
        let ctx = ScanContext::new(env, false);
        let t = huggingface_targets(&ctx);
        assert_eq!(t.len(), 1);
        assert_eq!(
            t[0].label.as_deref(),
            Some("meta-llama/Llama-3.2-1B (Model)")
        );
    }

    #[test]
    fn model_names() {
        assert_eq!(
            model_name(Path::new(r"registry.ollama.ai\library\llama3\8b")).unwrap(),
            "llama3:8b"
        );
        assert_eq!(
            model_name(Path::new(r"registry.ollama.ai\user\m\latest")).unwrap(),
            "user/m:latest"
        );
        assert_eq!(
            model_name(Path::new(r"hf.co\org\m\q4")).unwrap(),
            "hf.co/org/m:q4"
        );
    }
}
