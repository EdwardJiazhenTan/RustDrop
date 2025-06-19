use anyhow::Result;
use qrcode::QrCode;
use qrcode::render::unicode;

pub fn generate_qr_code(url: &str) -> Result<String> {
    let code = QrCode::new(url.as_bytes())?;
    let qr = code.render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    
    let mut output = String::new();
    output.push_str("\n");
    output.push_str("Scan this QR code to access RustDrop:\n");
    output.push_str(&qr);
    output.push_str("\n");
    output.push_str(&format!("Or open: {}\n", url));
    
    Ok(output)
}
