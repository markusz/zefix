// Moving scenarios in and out: a link that restores everything, TOML for the command line and
// AI assistants, and importing either.

import { useEffect, useRef, useState } from "react";
import { core } from "../core";
import type { Model } from "../model";
import { CloseIcon } from "./Icons";

export function shareLink(model: Model): string {
  const fragment = core.linkFragment(model);
  const base = `${location.origin}${location.pathname}`;
  return "ok" in fragment ? `${base}#${fragment.ok}` : base;
}

type Message = { kind: "info" | "error"; text: string } | null;

export function ShareDialog({
  open,
  model,
  onClose,
  onImport,
  initialMessage,
}: {
  open: boolean;
  model: Model;
  onClose: () => void;
  onImport: (model: Model) => void;
  initialMessage?: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [text, setText] = useState("");
  const [message, setMessage] = useState<Message>(
    initialMessage ? { kind: "error", text: initialMessage } : null,
  );

  useEffect(() => {
    const d = dialog.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    if (!open && d.open) d.close();
  }, [open]);

  const toml = open ? core.toToml(model) : null;
  const copy = async (what: string, value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setMessage({ kind: "info", text: `${what} kopiert.` });
    } catch {
      setMessage({ kind: "error", text: `${what} konnte nicht kopiert werden.` });
    }
  };
  const importNow = () => {
    const result = core.importText(text);
    if ("err" in result) {
      setMessage({ kind: "error", text: result.err });
      return;
    }
    onImport(result.ok.model);
    setText("");
    const dropped = result.ok.dropped;
    setMessage({
      kind: "info",
      text: dropped.length ? `Übernommen. Nicht übernommen: ${dropped.join("; ")}.` : "Übernommen.",
    });
  };

  return (
    <dialog ref={dialog} className="dialog" onClose={onClose}>
      <header className="dialog-head">
        <h2>Teilen</h2>
        <button className="icon-button" onClick={onClose} aria-label="Schließen">
          <CloseIcon />
        </button>
      </header>
      <p className="muted">Der Link stellt alles genau so wieder her. TOML ist das Format der Kommandozeile und des MCP-Servers.</p>
      <div className="row">
        <button className="button primary" onClick={() => copy("Link", shareLink(model))}>
          Link kopieren
        </button>
        {toml && "ok" in toml && (
          <button className="button" onClick={() => copy("TOML", toml.ok)}>
            TOML kopieren
          </button>
        )}
      </div>
      {toml && "ok" in toml && (
        <details>
          <summary>TOML ansehen</summary>
          <pre className="code-box">{toml.ok}</pre>
        </details>
      )}
      <h3>Importieren</h3>
      <textarea
        className="code"
        rows={6}
        placeholder="Link oder TOML einfügen"
        spellCheck={false}
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      <div className="row">
        <button
          className="button"
          disabled={!text.trim()}
          title="Ersetzt das aktuelle Szenario und alle Abfragen."
          onClick={importNow}
        >
          Übernehmen
        </button>
      </div>
      {message && <p className={message.kind === "error" ? "error-text" : "muted"}>{message.text}</p>}
    </dialog>
  );
}
