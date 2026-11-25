# schafsim

Monte-Carlo-Simulation für Schafkopf (langes Blatt). Eine TOML-Datei legt Spiel, eigene Hand und Fragen fest; `schafsim` gibt für jede Frage den Anteil der Deals aus, in denen sie zutrifft.

```
$ schafsim run scenarios/ober.toml
Sauspiel auf die Schellen-Sau · 10.000.000 Deals · Seed 42 · 0,21 s
Akzeptanz 57,14 % (17.502.225 Versuche)

┌───────────────────────────────────────┬─────────┬───────┬───────────┐
│ Abfrage                               ┆       P ┆ ±95 % ┆   Treffer │
╞═══════════════════════════════════════╪═════════╪═══════╪═══════════╡
│ Ein Gegner hat beide Ober + ≥2 Trumpf ┆ 16,48 % ┆  0,02 ┆ 1.647.644 │
│ Partner hat ≥3 Trumpf                 ┆ 46,74 % ┆  0,03 ┆ 4.673.678 │
│ Partner hat einen der beiden Ober     ┆ 52,57 % ┆  0,03 ┆ 5.256.528 │
└───────────────────────────────────────┴─────────┴───────┴───────────┘
```

## Aufbau

| Crate | Inhalt |
|---|---|
| `crates/core` (`schafsim-core`) | Spielregeln, Abfragesprache, Simulation, Statistik |
| `crates/cli` (`schafsim-cli`) | Kommandozeile `schafsim` |
| `crates/app` (`schafsim-app`) | Oberfläche für Desktop und Browser: Bedingungen und Abfragen per Baustein, Ergebnisse live |

Installieren: `cargo install --path crates/cli` · Oberfläche starten: `cargo run --release -p schafsim-app`

Im Browser (braucht `rustup target add wasm32-unknown-unknown` und [trunk](https://trunkrs.dev)): `cd crates/app && trunk serve --release`, dann http://127.0.0.1:8080 öffnen. `trunk build --release` legt die statischen Dateien in `crates/app/dist`. Im Browser rechnet die Simulation auf einem Kern, das Ergebnis ist bei gleichem Seed dasselbe wie auf dem Desktop.

## CLI

```
schafsim run <config> [-n 1e6] [--seed N] [--format text|json] [--examples N] [--threads N]
schafsim check <config>
```

`±95 %` ist die halbe Breite des 95-%-Wilson-Intervalls. Gleicher Seed → gleiches Ergebnis, unabhängig von der Thread-Zahl.

## Config

```toml
iterations = 10_000_000         # optional, Default 1e7
seed = 42                       # optional, sonst zeitbasiert (wird ausgegeben)

[game]
type = "sauspiel"               # sauspiel | solo | wenz | farbwenz | geier | farbgeier
called = "schellen"             # sauspiel: gerufene Sau
# trump = "herz"                # solo, farbwenz, farbgeier

[table]                         # optional
vorhand = "left"                # me | left | across | right

[me]
cards = ["EO", "GO"]            # feste Karten
slots = [                       # jeweils genau `count` Karten aus `from`
  { from = "unter",       count = 2 },
  { from = "herz",        count = 2 },
  { from = "farbe - sau", count = 2 },
]
where = "any(f in farben, me.count(f) == 2)"   # optionaler Filter auf den Deal

[[query]]
name  = "Ein Gegner hat beide Ober + ≥2 Trumpf"
where = "any(p in others, p.has(HO, SO) and p.count(trumpf - ober) >= 2)"
```

**Hand:** Slots sind disjunkt und zählen zusätzlich zu den festen Karten. Fehlen Karten bis 8, kommen sie aus einem impliziten Rest-Slot (alle Karten, die in keinem Slot vorkommen). Jede passende Hand ist gleich wahrscheinlich. `where` verwirft Deals; die Akzeptanzrate wird ausgegeben.

**Rufregel:** Im Sauspiel hat `me` die gerufene Sau nicht, aber mindestens eine Karte der Farbe. Deals, die das verletzen, werden verworfen. In v1 ist `me` immer der Spielmacher.

## Sprache

|                   |                                                                                                                           |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Karten            | `EO` `GU` `HA` `S10` (auch `SZ`) `EK` `G9` …                                                                              |
| Mengen            | `trumpf` `farbe` `alle` · `ober` `unter` `sau` `zehn` `koenig` `neun` `acht` `sieben` · `eichel` `gras` `herz` `schellen` |
| Mengen-Operatoren | `\|` Vereinigung, `&` Schnitt, `-` ohne                                                                                   |
| Spieler           | `me` `left` `across` `right` · `partner` (Sauspiel) · `vorhand` (mit `[table]`)                                           |
| Methoden          | `p.count(M)` · `p.has(K, …)` alle · `p.has_any(M)` · `p.frei(M)`                                                          |
| Quantoren         | `any` `all` `none` `(x in G, …)` → Wahrheitswert · `num(x in G, …)` → Zahl                                                |
| Gruppen           | `others` `opponents` `players` (Spieler) · `farben` `suits` (Farben)                                                      |
| Logik             | `and` `or` `not` · `==` `!=` `<` `<=` `>` `>=` · `+` `-` · Klammern                                                       |

Farbnamen gelten spielabhängig: im Sauspiel ist `herz` = HA H10 HK H9 H8 H7, `eichel` ohne EO/EU; im Wenz gehört EO zu `eichel`. `farben` sind die Nicht-Trumpf-Farben, `opponents` im Sauspiel die beiden Nicht-Partner, sonst alle drei.

```
partner.count(trumpf) >= 3
none(p in opponents, p.has_any(HO | SO))
num(f in farben, me.frei(f)) >= 1
all(p in others, p.count(trumpf) <= me.count(trumpf))
```
