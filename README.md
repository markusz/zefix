<img src="crates/app/assets/logo/zefix.svg" alt="zefix!-Logo" width="96" align="right">

# zefix!

Wie wahrscheinlich ist das Blatt, das einem gleich den Tout versaut? zefix! rechnet es aus, per Monte-Carlo-Simulation für Schafkopf (langes Blatt).

**Ausprobieren:** https://zefix.markusziller.de – läuft komplett im Browser, nichts wird hochgeladen.

## So funktioniert's

1. **Szenario:** was du weißt oder annimmst – Spiel, deine Position, dein Blatt (feste Karten oder nur Bedingungen wie „2–5 Trumpf“), optional Bedingungen an die anderen („Links ist Gras-frei“).
2. **Simulation:** zefix! mischt millionenfach Blätter, die zum Szenario passen.
3. **Abfragen:** Fragen, die an jedes gemischte Blatt gestellt werden, z. B. „Hat ein Gegner SO und mindestens drei Trumpf?“.
4. **Ergebnis:** der Anteil der Blätter, auf die eine Abfrage zutrifft – also ihre Wahrscheinlichkeit unter den Annahmen des Szenarios.

## Oberfläche

1. Szenario oben: Spiel (Sauspiel, Solo, Wenz, …) und die eigene Position wählen.
2. Szenario links: das eigene Blatt aus festen Karten und/oder Bedingungen wie „2–5 weitere Trumpf“. Auch für die anderen Spieler lassen sich Bedingungen setzen, z. B. „Links hat keine Gras“.
3. Abfragen rechts: wer (ein Gegner, beide Gegner, Mitspieler, …) was hat. Alternativ als Text in der Abfragesprache (siehe unten).

Ergebnisse erscheinen live, jede Änderung startet die Rechnung neu. „Genauigkeit“ legt fest, wie viele Blätter gemischt werden (10.000 bis 10 Mio.).

Lokal starten: `cargo run --release -p zefix-app`

## Kommandozeile

Für Szenarien als Datei, Skripte oder Batch-Läufe. Eine TOML-Datei legt Szenario und Abfragen fest. Typische Situationen und wie man sie ausdrückt: [`docs/beispiele.md`](docs/beispiele.md); weitere Szenarien in [`scenarios/`](scenarios).

```
$ cargo install --path crates/cli
$ zefix run scenarios/ober.toml
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

### Aufruf

```
zefix run <config> [-n 1e6] [--seed N] [--format text|json] [--examples N] [--threads N]
zefix check <config>
```

`±95 %` ist die halbe Breite des 95-%-Wilson-Intervalls. Gleicher Seed → gleiches Ergebnis, unabhängig von der Thread-Zahl.

### Config

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

## KI-Assistenten (MCP)

`zefix-mcp` stellt zefix! als [MCP](https://modelcontextprotocol.io)-Server bereit. Dann fragt man einfach: „Ich hab im Eichel-Solo EO, GO, HO, EU, GU, EA, E10, GA – wie oft hat ein Gegner SO mit drei Trumpf?“ Der Assistent schreibt daraus Szenario und Abfragen, rechnet und erklärt das Ergebnis.

```
cargo install --path crates/mcp
claude mcp add zefix -- zefix-mcp        # Claude Code
```

Andere Clients (z. B. Claude Desktop) bekommen `{"mcpServers": {"zefix": {"command": "/pfad/zu/.cargo/bin/zefix-mcp"}}}`. Tools: `reference` (Config-Format und Abfragesprache), `check` (Szenario prüfen), `simulate` (rechnen, Standard 1 Mio. Blätter).

## Abfragesprache

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

## Entwicklung

| Crate | Inhalt |
|---|---|
| `crates/core` (`zefix-core`) | Spielregeln, Abfragesprache, Simulation, Statistik |
| `crates/cli` (`zefix-cli`) | Kommandozeile `zefix` |
| `crates/app` (`zefix-app`) | Oberfläche für Desktop und Browser |
| `crates/mcp` (`zefix-mcp`) | MCP-Server für KI-Assistenten |

Die Oberfläche übersetzt ihre Bausteine in die Abfragesprache; Oberfläche und TOML haben also dieselbe Semantik. Regeln fürs Mitarbeiten stehen in [`AGENTS.md`](AGENTS.md).

- Tests: `cargo test`; Simulationsergebnisse werden in `crates/core/tests/closed_form.rs` gegen exakte hypergeometrische Werte geprüft.
- Browser lokal (braucht `rustup target add wasm32-unknown-unknown` und [trunk](https://trunkrs.dev)): `cd crates/app && trunk serve --release`, dann http://127.0.0.1:8080. Im Browser rechnet die Simulation auf einem Kern, kommt aber auf dieselben Zahlen wie der Desktop.
- Deployment: Jeder Push auf `develop` wird von Vercel gebaut (`vercel.json`, `scripts/vercel-build.sh`) und auf https://zefix.markusziller.de veröffentlicht.
