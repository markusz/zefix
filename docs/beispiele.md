# Beispiele

Typische Situationen am Tisch und wie man sie in zefix! ausdrückt. Jedes Beispiel ist eine vollständige Config: als Datei für `zefix run`, als Vorlage für einen KI-Assistenten über MCP, und in der Oberfläche mit denselben Bausteinen nachbaubar. Die Ergebnisse stammen aus 1 Mio. Blättern mit Seed 42.

Die Grundregel: **Was du weißt oder annimmst, gehört ins Szenario** (`[me] cards`, `slots`, `where`). **Was du wissen willst, wird eine Abfrage** (`[[query]]`). Jede Wahrscheinlichkeit bezieht sich nur auf die Blätter, die zum Szenario passen.

## 1. Läuft die Rufsau?

Sauspiel auf die Gras-Sau, ich habe nur die G7. Wenn ich Gras anspiele: Haben beide Gegner Gras, sodass keiner sticht?

```toml
[game]
type   = "sauspiel"
called = "gras"

[me]
cards = ["EO", "HO", "GU", "SU", "HA", "H9", "G7", "S10"]

[[query]]
name  = "Rufsau läuft: beide Gegner haben Gras"
where = "all(p in opponents, p.has_any(gras))"
```

**Ergebnis:** 69,73 %. `opponents` sind im Sauspiel die beiden Spieler außer mir und meinem Partner, wer immer das im jeweiligen Blatt ist.

## 2. Blatt nur teilweise bekannt

Ich überlege, ob sich ein Herz-Solo mit EO, GO, zwei Untern und drei Herz lohnt, egal welche Unter und welche Herz. Wie oft hat ein Gegner trotzdem vier Trumpf?

```toml
[game]
type  = "solo"
trump = "herz"

[me]
cards = ["EO", "GO"]
slots = [
  { from = "unter", count = 2 },
  { from = "herz",  count = 3 },
]

[[query]]
name  = "Ein Gegner hat ≥ 4 Trumpf"
where = "any(p in others, p.count(trumpf) >= 4)"
```

**Ergebnis:** 38,18 %. `slots` ziehen jeweils genau `count` Karten aus der Menge. Die achte Karte ist frei: Sie kommt aus allem, was in keinem Slot vorkommt. Wer alle Hände ausschließen will, die zusätzliche Trumpf haben, schreibt `where = "me.count(trumpf) == 7"` dazu.

## 3. Wie oft bekomme ich so ein Blatt überhaupt?

Ohne feste Karten ist auch die eigene Hand zufällig. Abfragen können sich dann auf `me` beziehen.

```toml
[game]
type  = "solo"
trump = "herz"

[[query]]
name  = "Ich habe ≥ 6 Trumpf"
where = "me.count(trumpf) >= 6"

[[query]]
name  = "Ich habe ≥ 6 Trumpf, davon ≥ 2 Ober"
where = "me.count(trumpf) >= 6 and me.count(ober) >= 2"
```

**Ergebnis:** 4,98 % und 3,06 %. Ungefähr jedes zwanzigste Blatt hat sechs Herz-Trumpf.

## 4. Bedingung oder Frage?

Wenz mit EU und GU. Links hat auf Herz nicht bedient, ist also Herz-frei. Wo stehen jetzt HU und SU?

Was man beim Spielen gesehen hat, ist Teil des Szenarios und kommt in `[me] where`. Dann rechnet zefix! nur mit Blättern, in denen links Herz-frei ist:

```toml
[game]
type = "wenz"

[me]
cards = ["EU", "GU", "EA", "E10", "EK", "GA", "G10", "SA"]
where = "left.frei(herz)"

[[query]]
name  = "Links hat HU oder SU"
where = "left.has_any(HU | SU)"

[[query]]
name  = "Ein anderer hat HU und SU"
where = "across.has(HU, SU) or right.has(HU, SU)"
```

**Ergebnis:** Links hat HU oder SU mit 73,58 %. HU und SU zusammen bei einem der anderen beiden: 12,36 %.

Zum Vergleich: dieselben Ausdrücke als Abfragen, ohne das Wissen über Herz.

```toml
[game]
type = "wenz"

[me]
cards = ["EU", "GU", "EA", "E10", "EK", "GA", "G10", "SA"]

[[query]]
name  = "Links ist Herz-frei"
where = "left.frei(herz)"

[[query]]
name  = "Links hat HU oder SU"
where = "left.has_any(HU | SU)"
```

**Ergebnis:** Links ist nur in 3,28 % der Blätter Herz-frei. Ohne dieses Wissen hat links HU oder SU mit 56,48 %, mit diesem Wissen mit 73,58 %. Der Unterschied ist genau die Information, die das Nicht-Bedienen liefert. Die Akzeptanzrate in der Ausgabe (hier ebenfalls 3,3 %) zeigt, wie selten das Szenario ist.

## 5. Partner und Laufende

Sauspiel auf die Eichel-Sau. Mit Partner zusammen: Haben wir die ersten drei Laufenden? Wie viele Trumpf haben wir gemeinsam?

```toml
[game]
type   = "sauspiel"
called = "eichel"

[me]
cards = ["EO", "HU", "SU", "HA", "HK", "H8", "E9", "S7"]

[[query]]
name  = "Mit Partner die ersten drei Laufenden (EO GO HO)"
where = "partner.has(GO, HO)"

[[query]]
name  = "Zusammen ≥ 9 Trumpf"
where = "me.count(trumpf) + partner.count(trumpf) >= 9"

[[query]]
name  = "Ein Gegner hat mehr Trumpf als mein Partner"
where = "any(p in opponents, p.count(trumpf) > partner.count(trumpf))"
```

**Ergebnis:** 8,32 %, 46,81 % und 61,97 %. `partner` ist der Spieler mit der gerufenen Sau. Zahlen lassen sich addieren und zwischen Spielern vergleichen.

## 6. Sitzordnung und Zählen über Spieler

Schellen-Solo, gegenüber spielt aus. Hat Vorhand beide fehlenden Unter? Und wie oft sitzen die acht restlichen Trumpf bei zwei Gegnern mit je mindestens drei?

```toml
[game]
type  = "solo"
trump = "schellen"

[table]
vorhand = "across"

[me]
cards = ["EO", "GO", "HO", "SO", "EU", "SA", "S10", "SK"]

[[query]]
name  = "Vorhand hat beide restlichen Unter"
where = "vorhand.has(GU, HU)"

[[query]]
name  = "Zwei Gegner haben je ≥ 3 Trumpf"
where = "num(p in others, p.count(trumpf) >= 3) >= 2"
```

**Ergebnis:** 10,15 % und 6,99 %. `vorhand` gibt es nur mit `[table]`. `num(p in others, …)` zählt, für wie viele Spieler eine Bedingung gilt.

## 7. Mengen kombinieren

Gras-Solo. Fragen, die mit Teilmengen arbeiten: Trumpf ohne Ober, Zehnen einer Farbe, Sau-Durchläufe.

```toml
[game]
type  = "solo"
trump = "gras"

[me]
cards = ["EO", "GO", "HO", "EU", "GA", "G10", "GK", "EA"]

[[query]]
name  = "Ein Gegner hat ≥ 3 Trumpf ohne Ober"
where = "any(p in others, p.count(trumpf - ober) >= 3)"

[[query]]
name  = "Eine fremde Zehn steht blank"
where = "any(p in others, any(f in farben, p.has_any(f & zehn) and p.count(f) == 1))"

[[query]]
name  = "Eichel-Sau geht durch: alle haben Eichel"
where = "all(p in others, p.has_any(eichel))"
```

**Ergebnis:** 83,66 %, 39,87 % und 69,51 %. `-` nimmt Karten aus einer Menge heraus, `&` schneidet zwei Mengen, `|` vereinigt sie. `any(f in farben, …)` läuft über die Farben, die in diesem Spiel nicht Trumpf sind.

## 8. Andere Spiele, andere Trumpf

Im Wenz und Farbwenz sind Ober normale Farbkarten. Im Eichel-Wenz ist der EO also Eichel und damit Trumpf.

```toml
[game]
type  = "farbwenz"
trump = "eichel"

[me]
cards = ["EU", "GU", "EA", "E10", "EK", "EO", "GA", "HA"]

[[query]]
name  = "HU und SU beim selben Gegner"
where = "any(p in others, p.has(HU, SU))"

[[query]]
name  = "Ein Gegner hat ≥ 3 der 5 übrigen Trumpf"
where = "any(p in others, p.count(trumpf) >= 3)"
```

**Ergebnis:** 30,40 % und 55,60 %. Ich habe sechs Trumpf (EU GU EA E10 EK EO). Übrig sind HU, SU, E9, E8 und E7. Mengen wie `trumpf`, `eichel` oder `ober` richten sich immer nach dem Spiel.

## Mehr

In [`scenarios/`](../scenarios) liegen weitere Szenarien. Am ausführlichsten ist [`eichel_solo_tout.toml`](../scenarios/eichel_solo_tout.toml): Dort wird durchgerechnet, wann ein Tout am Anspiel von Vorhand scheitert.
