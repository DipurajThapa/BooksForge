//! Post-build KDP eBook structural checks (BACKLOG §G3).
//!
//! Runs against an in-memory EPUB byte stream and returns findings about
//! cover image, total file size, nav-document health, and embedded image
//! sizes — the things KDP rejects or auto-rejects on upload.  Pre-export
//! metadata checks (title / author / ISBN / language) live in
//! `booksforge-validator::validators::kdp_metadata`; this module
//! complements them with structural checks that can only be run once
//! the archive is built.

use std::io::{Cursor, Read};

/// Severity is intentionally a plain enum here so this module stays
/// dependency-free of `booksforge-domain` (no cycle risk) and the
/// caller maps it to whatever error/warning surface they expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdpSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct KdpFinding {
    pub severity: KdpSeverity,
    pub code: &'static str,
    pub message: String,
}

/// KDP recommended ceiling for fast-delivery royalty (50 MB).
/// Above this the file is still accepted but delivery cost reduces
/// 70 % royalty to 35 % on Amazon's tiered scheme.
const KDP_RECOMMENDED_MAX_BYTES: u64 = 50 * 1024 * 1024;

/// KDP hard upload limit (650 MB) — files larger than this are rejected.
const KDP_HARD_MAX_BYTES: u64 = 650 * 1024 * 1024;

/// Per-image soft cap; KDP's reflow renderer downsamples larger.
const KDP_IMAGE_SOFT_MAX_BYTES: u64 = 5 * 1024 * 1024;

/// Inspect the built EPUB byte stream and return KDP-specific findings.
///
/// Findings only — does not mutate, does not write.  Safe to run on
/// any bytes that pass the basic ZIP smoke test; bytes that fail to
/// open as a ZIP are reported as a single error finding so the caller
/// can surface "this isn't a valid EPUB at all".
pub fn run_kdp_checks(bytes: &[u8]) -> Vec<KdpFinding> {
    let mut out = Vec::new();
    let total = bytes.len() as u64;

    // ── Total size band ──
    if total > KDP_HARD_MAX_BYTES {
        out.push(KdpFinding {
            severity: KdpSeverity::Error,
            code: "KDP06",
            message: format!(
                "EPUB is {:.1} MB — KDP rejects files over 650 MB.",
                total as f64 / (1024.0 * 1024.0)
            ),
        });
    } else if total > KDP_RECOMMENDED_MAX_BYTES {
        out.push(KdpFinding {
            severity: KdpSeverity::Warning,
            code: "KDP07",
            message: format!(
                "EPUB is {:.1} MB — KDP delivery cost reduces royalty above 50 MB.",
                total as f64 / (1024.0 * 1024.0)
            ),
        });
    }

    // ── Open as ZIP ──
    let mut zip = match zip::ZipArchive::new(Cursor::new(bytes)) {
        Ok(z) => z,
        Err(e) => {
            out.push(KdpFinding {
                severity: KdpSeverity::Error,
                code: "KDP08",
                message: format!("Built EPUB cannot be re-opened as a ZIP: {e}"),
            });
            return out;
        }
    };

    // Walk entries once, harvesting:
    //   - the OPF document (for cover-image + spine/nav references)
    //   - the nav document (for TOC presence)
    //   - per-image entry sizes
    let mut opf_xml: Option<String> = None;
    let mut nav_xml: Option<String> = None;
    let mut large_images: Vec<(String, u64)> = Vec::new();

    for i in 0..zip.len() {
        let mut entry = match zip.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().to_owned();
        let size = entry.size();

        // Per-image soft cap (KDP08 used above for ZIP error; image cap
        // surfaces as KDP09).
        let lower = name.to_ascii_lowercase();
        if (lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".png")
            || lower.ends_with(".gif")
            || lower.ends_with(".webp"))
            && size > KDP_IMAGE_SOFT_MAX_BYTES
        {
            large_images.push((name.clone(), size));
        }

        // Capture OPF + nav as text — small, cheap.
        if lower.ends_with(".opf") {
            let mut s = String::new();
            if entry.read_to_string(&mut s).is_ok() {
                opf_xml = Some(s);
            }
        } else if lower.ends_with("nav.xhtml") {
            let mut s = String::new();
            if entry.read_to_string(&mut s).is_ok() {
                nav_xml = Some(s);
            }
        }
    }

    for (name, sz) in &large_images {
        out.push(KdpFinding {
            severity: KdpSeverity::Warning,
            code:     "KDP09",
            message:  format!(
                "Image '{name}' is {:.1} MB — KDP downsamples images above 5 MB which can degrade quality.",
                *sz as f64 / (1024.0 * 1024.0)
            ),
        });
    }

    // ── Cover image declared in OPF? ──
    // EPUB-3 cover convention: a manifest item carries
    // `properties="cover-image"`.  We string-match because pulling in a
    // real XML parser for one substring check is overkill.
    match opf_xml.as_deref() {
        Some(xml) if xml.contains("properties=\"cover-image\"") => { /* ok */ }
        Some(_) => out.push(KdpFinding {
            severity: KdpSeverity::Warning,
            code: "KDP10",
            message: "No cover image declared in the EPUB.  KDP will fall back to a generated \
                 placeholder; bundle a cover via assets/cover.{jpg,png} for retail listings."
                .into(),
        }),
        None => out.push(KdpFinding {
            severity: KdpSeverity::Error,
            code: "KDP11",
            message: "No OPF package document found in the EPUB — the archive is malformed.".into(),
        }),
    }

    // ── Nav document health ──
    // EPUB-3 requires `nav.xhtml` with at least one
    // `<nav epub:type="toc">` element listing the spine.
    match nav_xml.as_deref() {
        Some(xml) if xml.contains("epub:type=\"toc\"") => { /* ok */ }
        Some(_) => out.push(KdpFinding {
            severity: KdpSeverity::Warning,
            code: "KDP12",
            message: "nav.xhtml is present but lacks a `<nav epub:type=\"toc\">` element — \
                 KDP readers fall back to an inferred TOC which may misorder chapters."
                .into(),
        }),
        None => out.push(KdpFinding {
            severity: KdpSeverity::Error,
            code: "KDP13",
            message: "nav.xhtml missing — EPUB-3 (and KDP) require a navigation document.".into(),
        }),
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_zip_bytes() {
        let findings = run_kdp_checks(b"not a zip");
        assert!(findings.iter().any(|f| f.code == "KDP08"));
    }

    #[test]
    fn flags_oversized_image_entries() {
        // Build a tiny ZIP containing a fake 6 MB JPEG entry.
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            // OPF stub with a cover-image declaration so KDP10 doesn't
            // also fire — we want a single-finding test.
            w.start_file("OEBPS/content.opf", opts).unwrap();
            use std::io::Write as _;
            w.write_all(
                b"<?xml version=\"1.0\"?><package><manifest>\
                  <item href=\"cover.jpg\" properties=\"cover-image\" id=\"c\" media-type=\"image/jpeg\"/>\
                  </manifest></package>",
            ).unwrap();
            w.start_file("OEBPS/big.jpg", opts).unwrap();
            w.write_all(&vec![0u8; (KDP_IMAGE_SOFT_MAX_BYTES + 1) as usize])
                .unwrap();
            w.start_file("OEBPS/nav.xhtml", opts).unwrap();
            w.write_all(b"<html><body><nav epub:type=\"toc\"></nav></body></html>")
                .unwrap();
            w.finish().unwrap();
        }
        let findings = run_kdp_checks(&buf);
        assert!(
            findings.iter().any(|f| f.code == "KDP09"),
            "expected KDP09 for oversized image, got: {findings:#?}"
        );
    }

    #[test]
    fn flags_missing_cover_and_nav() {
        // Build an empty-but-valid ZIP — no OPF, no nav.
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            w.start_file("placeholder.txt", opts).unwrap();
            use std::io::Write as _;
            w.write_all(b"placeholder").unwrap();
            w.finish().unwrap();
        }
        let findings = run_kdp_checks(&buf);
        assert!(
            findings.iter().any(|f| f.code == "KDP11"),
            "missing OPF should flag KDP11"
        );
        assert!(
            findings.iter().any(|f| f.code == "KDP13"),
            "missing nav should flag KDP13"
        );
    }

    #[test]
    fn passes_clean_kdp_friendly_archive() {
        // OPF declares cover-image, nav has toc element, no big images.
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            use std::io::Write as _;
            w.start_file("OEBPS/content.opf", opts).unwrap();
            w.write_all(
                b"<package><manifest>\
                  <item href=\"cover.jpg\" properties=\"cover-image\" id=\"c\" media-type=\"image/jpeg\"/>\
                  </manifest></package>",
            ).unwrap();
            w.start_file("OEBPS/nav.xhtml", opts).unwrap();
            w.write_all(b"<nav epub:type=\"toc\"><ol><li>ch1</li></ol></nav>")
                .unwrap();
            w.finish().unwrap();
        }
        let findings = run_kdp_checks(&buf);
        // Allow info-level findings; warnings/errors must be empty.
        assert!(
            findings.iter().all(|f| f.severity == KdpSeverity::Info),
            "expected no warnings/errors, got: {findings:#?}",
        );
    }
}
