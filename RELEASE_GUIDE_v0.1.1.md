# Guía de Release — v0.1.1

> **Archivo temporal** — eliminar después de completar el release.

---

## Prerequisitos

- Estás en la rama `main` con los cambios del fix de CI ya commiteados (`02dc6af`).
- `CHANGELOG.md` y `release.yml` ya fueron actualizados (hecho ✅).
- El `Cargo.toml` todavía tiene `version = "0.1.0"` — hay que actualizarlo primero.

---

## Paso 1 — Actualizar versión en `Cargo.toml`

Abre `Cargo.toml` y cambia:

```toml
version = "0.1.0"
```

por:

```toml
version = "0.1.1"
```

---

## Paso 2 — Crear el commit de release

En RustRover: **Git → Commit** (o `Ctrl+K`)

Usa exactamente este mensaje de commit (Conventional Commits):

```
chore(release): bump version to v0.1.1

- Update Cargo.toml version to 0.1.1
- Update CHANGELOG.md with v0.1.1 entries (UX + scope + CI fixes)
- Fix release workflow: remove dynamic linker config (already in
  .cargo/config.toml), add retention-days to artifacts, add Raspberry Pi
  install instructions
```

> **Reglas:** imperativo, sin mayúscula, sin punto final en subject, subject ≤ 50 chars.

---

## Paso 3 — Crear el tag `v0.1.1`

En RustRover: **Git → Tags → Create Tag…**  
O desde el terminal integrado (**Alt+F12**):

```bash
git tag -a v0.1.1 -m "chore(release): v0.1.1 — UX and scope refinement"
```

---

## Paso 4 — Push commit + tag

En RustRover: **Git → Push** (o `Ctrl+Shift+K`)

En el diálogo de Push asegúrate de marcar **"Push Tags"** → selecciona `v0.1.1`.

O desde terminal:

```bash
git push origin main --follow-tags
```

---

## Paso 5 — Verificar el workflow en GitHub Actions

1. Ve a `https://github.com/CoreRed-Project/orph-cli/actions`
2. Verifica que el workflow **Release** se disparó con el tag `v0.1.1`
3. Jobs que deben pasar:
   - `Build Linux ARM64 (Raspberry Pi)`
   - `Create GitHub Release`

---

## Paso 6 — Verificar el Release en GitHub

1. Ve a `https://github.com/CoreRed-Project/orph-cli/releases/tag/v0.1.1`
2. Confirma que están adjuntos los 2 binarios:
   - `orph-linux-aarch64`
   - `orphd-linux-aarch64`
3. Verifica que el body del release contiene las instrucciones de install para Raspberry Pi.

---

## Release Notes sugeridas

Copia esto en el campo de descripción del release si quieres personalizar más allá del `generate_release_notes` automático:

```markdown
## v0.1.1 — UX and scope refinement

This release focuses on UX polish and scope clarity for `orph` as a
local-first cyberdeck CLI tooling system.

### Changes
- Scope redefined: `orphd` is now an optional state accelerator, not
  a required component — `orph` is fully functional offline-first
- Human-readable output reviewed across all commands for consistency
- Daemon offline banner deduplicated (shown once per invocation)
- Actionable error hints improved for `core start`, `run`, and `cfg set`

### Fixed
- ARM64 cross-compilation in CI: removed fragile dynamic linker append,
  linker config now lives statically in `.cargo/config.toml`
- Clippy: `checked_div()` for `mem_pct`/`disk_pct`, `RunFlags` type alias

### Install


**Raspberry Pi (ARM64):**
curl -L .../orph-linux-aarch64 -o orph && curl -L .../orphd-linux-aarch64 -o orphd
chmod +x orph orphd && sudo mv orph orphd /usr/local/bin/
```

---

> Eliminar este archivo después de completar el release.

