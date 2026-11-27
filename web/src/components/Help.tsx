// Frequently asked questions, in a dialog reachable from the brand strip.

import { useEffect, useRef } from "react";
import { CloseIcon } from "./Icons";

const REPO = "https://github.com/markusz/zefix";

const FAQ: { q: string; a: React.ReactNode }[] = [
  {
    q: "Was rechnet zefix! aus?",
    a: (
      <>
        Wie oft etwas eintritt, wenn die Karten so liegen, wie du es unter <b>Blätter</b> beschreibst.
        zefix! mischt dafür viele zufällige Blätter, behält die, die zu deinen Angaben passen, und
        zählt, wie oft jede <b>Abfrage</b> zutrifft. Der Anteil ist die Wahrscheinlichkeit.
      </>
    ),
  },
  {
    q: "Was ist der Unterschied zwischen Karte und Regel?",
    a: (
      <>
        Eine <b>Karte</b> liegt fest auf der Hand, z. B. der Gras-Ober. Eine <b>Regel</b> sagt nur, wie
        viele Karten einer Sorte es sind, z. B. „mindestens 3 Trumpf“ oder „keine Herz“. Hände ohne
        Angaben werden zufällig gemischt.
      </>
    ),
  },
  {
    q: "Zählen Regeln zu den festen Karten dazu?",
    a: (
      <>
        Ja. Regeln zählen zusätzlich zu den festen Karten derselben Hand: EO, GO und „2–5 Trumpf“ heißt
        zwei bis fünf <i>weitere</i> Trumpf, also vier bis sieben insgesamt.
      </>
    ),
  },
  {
    q: "Wer ist mein Mitspieler, wer sind die Gegner?",
    a: (
      <>
        Im Sauspiel spielst du und rufst die gewählte Sau. Mitspieler ist, wer sie hat; die anderen
        beiden sind die Gegner. Nach der Rufregel hast du die gerufene Sau nicht, aber mindestens eine
        Karte ihrer Farbe. In allen anderen Spielen spielst du allein gegen die drei anderen.
      </>
    ),
  },
  {
    q: "Wie genau sind die Zahlen?",
    a: (
      <>
        Das Ergebnis ist eine Schätzung aus gemischten Blättern; „Genauigkeit“ legt fest, wie viele.
        Mit 95 % Sicherheit liegt der wahre Wert höchstens so weit daneben: Sehr schnell (10.000)
        1 Prozentpunkt, Schnell (100.000) 0,3, Normal (1 Mio.) 0,1, Genau (10 Mio.) 0,03.
      </>
    ),
  },
  {
    q: "Warum kommt bei gleichen Angaben immer dieselbe Zahl heraus?",
    a: (
      <>
        Gemischt wird mit festem Startwert. So unterscheiden sich zwei Varianten nur durch das, was du
        geändert hast, nicht durch Zufall, und Rückgängigmachen bringt das alte Ergebnis zurück.
      </>
    ),
  },
  {
    q: "Was heißt „davon passend“?",
    a: (
      <>
        Nicht jedes gemischte Blatt passt zu deinen Angaben oder zur Rufregel. Solche Blätter werden
        verworfen; „davon passend“ ist der Anteil, der übrig bleibt. Ist er sehr klein, rechnet zefix!
        länger.
      </>
    ),
  },
  {
    q: "Was kann ich als Text schreiben?",
    a: (
      <>
        Am Computer lässt sich jede Abfrage auf <b>Text</b> umstellen, und unter <b>Weitere Bedingung</b>{" "}
        geht, was Bausteine nicht können, etwa ein „oder“: <code>left.frei(gras) or right.has(SO)</code>.
        Die Abfragesprache und Beispiele stehen im{" "}
        <a href={`${REPO}#abfragesprache`} target="_blank" rel="noreferrer">
          README
        </a>{" "}
        und in den{" "}
        <a href={`${REPO}/blob/develop/docs/beispiele.md`} target="_blank" rel="noreferrer">
          Beispielen
        </a>
        .
      </>
    ),
  },
  {
    q: "Wird etwas hochgeladen?",
    a: (
      <>
        Nein. Alles läuft in deinem Browser. Der aktuelle Stand steht in der Adresszeile; wer den Link
        bekommt, sieht genau dasselbe.
      </>
    ),
  },
  {
    q: "Gibt es zefix! auch anders?",
    a: (
      <>
        Ja: als Mac-App, die auf allen Kernen rechnet, als Kommandozeile für Szenarien in TOML-Dateien
        und als MCP-Server für KI-Assistenten. Alles im{" "}
        <a href={REPO} target="_blank" rel="noreferrer">
          Repository
        </a>
        .
      </>
    ),
  },
];

export function HelpDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const d = dialog.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    if (!open && d.open) d.close();
  }, [open]);
  return (
    <dialog ref={dialog} className="dialog help" onClose={onClose}>
      <header className="dialog-head">
        <h2>Häufige Fragen</h2>
        <button className="icon-button" onClick={onClose} aria-label="Schließen">
          <CloseIcon />
        </button>
      </header>
      {FAQ.map(({ q, a }) => (
        <details key={q} className="faq">
          <summary>{q}</summary>
          <p>{a}</p>
        </details>
      ))}
    </dialog>
  );
}
