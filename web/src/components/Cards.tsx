// Card faces, rule cards and suit symbols, drawn as SVG on a 34 × 48 card.

import type { CardSet, CountOp, Suit } from "../model";
import { SUIT_NAMES, isNegation, opBadge } from "../model";

const SUIT_BY_LETTER: Record<string, Suit> = { E: "eichel", G: "gras", H: "herz", S: "schellen" };

export function cardSuit(card: string): Suit {
  return SUIT_BY_LETTER[card[0] ?? "E"] ?? "eichel";
}

export function cardRank(card: string): string {
  return card.slice(1);
}

/** E.g. "Gras-Ober" for `GO`, for tooltips and screen readers. */
export function cardName(card: string): string {
  const ranks: Record<string, string> = {
    O: "Ober",
    U: "Unter",
    A: "Sau",
    "10": "Zehner",
    K: "König",
    "9": "Neuner",
    "8": "Achter",
    "7": "Siebener",
  };
  return `${SUIT_NAMES[cardSuit(card)]}-${ranks[cardRank(card)] ?? cardRank(card)}`;
}

export function SuitImage({ suit, x, y, size }: { suit: Suit; x: number; y: number; size: number }) {
  return (
    <image
      href={`${import.meta.env.BASE_URL}suits/${suit}.svg`}
      x={x - size / 2}
      y={y - size / 2}
      width={size}
      height={size}
      preserveAspectRatio="xMidYMid meet"
    />
  );
}

export function SuitIcon({ suit, size = 20 }: { suit: Suit; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 20 20" aria-hidden>
      <SuitImage suit={suit} x={10} y={10} size={19} />
    </svg>
  );
}

export function CardFace({ card }: { card: string }) {
  const rank = cardRank(card);
  return (
    <svg className="card-svg" viewBox="0 0 34 48" aria-hidden>
      <rect className="card-paper" x={0.75} y={0.75} width={32.5} height={46.5} rx={3.5} />
      <SuitImage suit={cardSuit(card)} x={17} y={16} size={22} />
      <text className="card-rank" x={17} y={43} textAnchor="middle" fontSize={rank === "10" ? 13 : 15}>
        {rank}
      </text>
    </svg>
  );
}

type RuleSymbol =
  | { kind: "suit"; suit: Suit }
  | { kind: "rank"; symbol: string }
  | { kind: "trumpf" }
  | { kind: "suits"; suits: Suit[] };

function ruleSymbol(set: CardSet, plainSuits: Suit[]): RuleSymbol {
  if (typeof set !== "string") return { kind: "suit", suit: set.Suit };
  switch (set) {
    case "Trumpf":
      return { kind: "trumpf" };
    case "Farbe":
      return { kind: "suits", suits: plainSuits };
    case "Ober":
      return { kind: "rank", symbol: "O" };
    case "Unter":
      return { kind: "rank", symbol: "U" };
    case "Sau":
      return { kind: "rank", symbol: "A" };
    case "Zehn":
      return { kind: "rank", symbol: "10" };
    case "Koenig":
      return { kind: "rank", symbol: "K" };
  }
}

const ALL_SUITS: Suit[] = ["eichel", "gras", "herz", "schellen"];

function Symbol({ symbol }: { symbol: RuleSymbol }) {
  switch (symbol.kind) {
    case "suit":
      return <SuitImage suit={symbol.suit} x={17} y={17} size={22} />;
    case "trumpf":
      return <Crown />;
    case "rank":
      return (
        <>
          <text className="card-rank" x={17} y={19} textAnchor="middle" fontSize={15}>
            {symbol.symbol}
          </text>
          {ALL_SUITS.map((suit, i) => (
            <SuitImage key={suit} suit={suit} x={6.5 + i * 7} y={27} size={6.5} />
          ))}
        </>
      );
    case "suits": {
      const n = symbol.suits.length;
      return (
        <>
          {symbol.suits.map((suit, i) => {
            const col = n <= 2 ? i : i % 2;
            const row = n <= 2 ? 0 : Math.floor(i / 2);
            const cols = Math.min(n, 2);
            const rows = Math.ceil(n / 2);
            const x = 17 + (col - (cols - 1) / 2) * 11;
            const y = 18 + (row - (rows - 1) / 2) * 11;
            return <SuitImage key={suit} suit={suit} x={x} y={y} size={10} />;
          })}
        </>
      );
    }
  }
}

function Crown() {
  return (
    <g className="crown" transform="translate(17 17)">
      <path className="crown-body" d="M-9 7 V-6 L-4.5 0 L0 -8 L4.5 0 L9 -6 V7 Z" />
      <rect className="crown-band" x={-9} y={3.5} width={18} height={3.5} />
      <circle className="crown-jewel" cx={-9} cy={-6} r={1.4} />
      <circle className="crown-jewel" cx={0} cy={-8} r={1.4} />
      <circle className="crown-jewel" cx={9} cy={-6} r={1.4} />
    </g>
  );
}

export function RuleFace({ set, op, plainSuits }: { set: CardSet; op: CountOp; plainSuits: Suit[] }) {
  const badge = opBadge(op);
  const badgeWidth = Math.max(12, badge.length * 5 + 6);
  return (
    <svg className="card-svg" viewBox="0 0 34 48" aria-hidden>
      <rect className="rule-paper" x={0.75} y={0.75} width={32.5} height={46.5} rx={3.5} />
      <Symbol symbol={ruleSymbol(set, plainSuits)} />
      {isNegation(op) && <line className="strike" x1={5} y1={6} x2={29} y2={29} />}
      <rect className="badge" x={17 - badgeWidth / 2} y={34} width={badgeWidth} height={9} rx={4.5} />
      <text className="badge-text" x={17} y={41} textAnchor="middle" fontSize={7}>
        {badge}
      </text>
    </svg>
  );
}
