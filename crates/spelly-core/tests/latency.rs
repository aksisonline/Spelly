use std::time::Instant;

use harper_core::Dialect;
use spelly_core::*;

/// The Windows hook waits for us on every word boundary, so this must stay fast.
#[test]
fn boundary_is_fast() {
    let mut s = Spelly::new(&[], Dialect::Indian);
    let text = "I was walking to the market yesterday and then I saw my freind there. We talked about the new anime that everyone keeps recomending to us teh ";
    s.on_boundary(text); // warm up
    let t = Instant::now();
    for _ in 0..20 {
        s.on_boundary(text);
    }
    let ms = t.elapsed().as_millis() / 20;
    println!("avg on_boundary: {ms} ms");
    let limit = if cfg!(debug_assertions) { 400 } else { 60 };
    assert!(ms < limit, "{ms} ms per boundary");
}
