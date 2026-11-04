//! Protect Using Password in the real shell (egui_kittest).

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::{Dialog, PdfCraftApp};

const FIXTURE: &[u8] = b"%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] >> endobj
trailer << /Root 1 0 R >>
%%EOF";

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("doc.pdf", None, FIXTURE.to_vec()).unwrap();
        app
    });
    h.run_steps(4);
    h
}

fn type_into(h: &mut Harness<'static, PdfCraftApp>, label: &str, text: &str) {
    h.get_by_label(label).click();
    h.run_steps(2);
    h.get_by_label(label).type_text(text);
    h.run_steps(2);
}

#[test]
fn protecting_for_viewing_requires_matching_passwords_and_saves_encrypted() {
    let mut h = harness();
    assert!(h.state_mut().execute("protect.password"));
    h.run_steps(2);
    assert_eq!(h.state().dialog, Some(Dialog::Protect));
    h.get_by_label("Viewing");
    type_into(&mut h, "Type Password", "Open-Sesame-42");
    type_into(&mut h, "Re-type Password", "Open-Sesame-4");
    h.get_by_label_contains("don't match");
    assert!(h.query_by(|n| n.label().as_deref() == Some("Apply") && n.is_disabled()).is_some(), "Apply is disabled");
    type_into(&mut h, "Re-type Password", "2");
    h.get_by_label("Strength: Strong");
    h.get_by_label("Apply").click();
    h.run_steps(3);
    assert_eq!(h.state().dialog, None);
    let id = h.state().views[0].id;
    let doc = h.state().session.get(id).unwrap();
    assert!(doc.info.encrypted && doc.dirty);
    assert!(doc.security_summary().unwrap().pending);
    // What Save writes needs the password.
    let bytes = h.state().session.save_bytes(id).unwrap();
    let mut s = pdfcraft_engine::Session::new();
    assert!(s.open("p.pdf", None, bytes.clone(), None).is_err());
    assert!(s.open("p.pdf", None, bytes, Some("Open-Sesame-42")).is_ok());
    // Remove security is now offered, and undo also takes the protection back.
    assert!(h.state_mut().execute("protect.remove"));
    h.state_mut().undo();
    h.state_mut().undo();
    assert!(!h.state().session.get(id).unwrap().info.encrypted);
}

#[test]
fn editing_restrictions_via_advanced_options() {
    let mut h = harness();
    h.state_mut().execute("protect.password");
    h.run_steps(2);
    h.get_by_label("Editing").click();
    h.run_steps(2);
    h.get_by_label("Advanced Options ⌄").click();
    h.run_steps(2);
    h.get_by_label("Enable copying of text, images, and other content").click();
    h.run_steps(1);
    type_into(&mut h, "Type Password", "boss");
    type_into(&mut h, "Re-type Password", "boss");
    h.get_by_label("Apply").click();
    h.run_steps(3);
    let id = h.state().views[0].id;
    let sec = h.state().session.get(id).unwrap().security_summary().unwrap();
    assert!(sec.permissions.copy() && sec.permissions.print() && !sec.permissions.modify() && !sec.permissions.annotate());
    // Remove security is disabled for nobody here: this session set it.
    assert!(h.state().session.get(id).unwrap().allows_security_change());
}

/// FIXTURE rewritten with an owner password and printing-only permissions (everything else
/// denied), as a viewer opens it: no password asked, nothing but printing allowed.
fn restricted() -> Vec<u8> {
    let mut doc = pdfcraft_cos::Document::open(std::sync::Arc::new(FIXTURE.to_vec())).unwrap();
    doc.set_encryption(&pdfcraft_cos::NewEncryption {
        algorithm: pdfcraft_cos::Algorithm::Aes256,
        user_password: "",
        owner_password: "owner",
        permissions: 0b0100,
        encrypt_metadata: true,
        seed: [7; 32],
    })
    .unwrap();
    pdfcraft_cos::write_full(&doc, &Default::default()).unwrap()
}

fn restricted_harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("locked.pdf", None, restricted()).unwrap();
        app
    });
    h.run_steps(4);
    h
}

/// Unlock PDF takes every restriction off without asking for the permissions password (the
/// web unlock tools' behaviour), and the save writes an unencrypted file.
#[test]
fn unlock_pdf_removes_the_restrictions_without_asking_for_the_password() {
    let mut h = restricted_harness();
    let id = h.state().views[0].id;
    assert!(!h.state().session.get(id).unwrap().allows_modification(), "restricted to start with");
    // Offered even though every ordinary change is refused.
    assert!(h.state_mut().execute("protect.unlock_document"));
    h.run_steps(2);
    assert_eq!(h.state().dialog, Some(Dialog::UnlockPdf));
    assert!(h.state().unlock_prompt.is_none(), "no password is asked for");
    h.get_by_label_contains("Printing, copying and changing the document will be allowed");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Unlock PDF").click();
    h.run_steps(3);
    assert_eq!(h.state().dialog, None, "the dialog closes after unlocking");
    let doc = h.state().session.get(id).unwrap();
    assert!(doc.dirty, "the unlocked file waits for a save");
    assert!(doc.permissions().is_none() && doc.allows_modification() && doc.allows_copying());
    // The save writes a file that opens without a password, restricted by nothing.
    let bytes = h.state().session.save_bytes(id).unwrap();
    assert!(pdfcraft_cos::Document::open(bytes).unwrap().security().is_none());
}

/// Cancelling the confirmation leaves the document exactly as it was.
#[test]
fn cancelling_unlock_pdf_changes_nothing() {
    let mut h = restricted_harness();
    let id = h.state().views[0].id;
    assert!(h.state_mut().execute("protect.unlock_document"));
    h.run_steps(2);
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Cancel").click();
    h.run_steps(2);
    assert_eq!(h.state().dialog, None);
    let doc = h.state().session.get(id).unwrap();
    assert!(!doc.dirty && !doc.allows_modification());
}
