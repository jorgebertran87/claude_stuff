fn main() {
    // Store adapters (Mercadona, Maskom, Dia, Carrefour, ...) are not wired
    // yet; the comparison domain lives in prices_comparer::comparer.
    eprintln!("prices_comparer: no store adapters wired yet");
    std::process::exit(1);
}
