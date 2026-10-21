//! Бенчмарки: анализ токена и построение словарей.

use caramba_core::engine::{Engine, Token};
use caramba_core::layouts::builtin_pair;
use criterion::{criterion_group, criterion_main, Criterion};

fn token_en(s: &str) -> Token {
    let pair = builtin_pair();
    let layout = &pair.en;
    let presses = s
        .chars()
        .filter_map(|c| {
            layout
                .keys
                .iter()
                .find(|e| e.lower == c.to_string() || e.upper == c.to_string())
                .map(|e| caramba_core::engine::Press {
                    keycode: e.kc,
                    shift: e.upper == c.to_string(),
                    caps: false,
                })
        })
        .collect();
    Token { presses }
}

fn bench_word_analysis(c: &mut Criterion) {
    let mut e = Engine::new();
    let token = token_en("ghbdtn");
    c.bench_function("analyze_ghbdtn", |b| b.iter(|| e.evaluate(&token, 1000)));
    let token2 = token_en("millionaire");
    c.bench_function("analyze_valid_en", |b| b.iter(|| e.evaluate(&token2, 1000)));
}

fn bench_dictionary_load(c: &mut Criterion) {
    c.bench_function("engine_new", |b| b.iter(Engine::new));
}

criterion_group!(benches, bench_word_analysis, bench_dictionary_load);
criterion_main!(benches);
