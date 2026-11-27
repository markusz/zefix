// Small shared inputs.

export function Stepper({
  value,
  min,
  max,
  onChange,
  prefix,
}: {
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
  prefix?: string;
}) {
  return (
    <span className="stepper">
      <button aria-label="weniger" disabled={value <= min} onClick={() => onChange(Math.max(min, value - 1))}>
        −
      </button>
      <span className="stepper-value">
        {prefix}
        {value}
      </span>
      <button aria-label="mehr" disabled={value >= max} onClick={() => onChange(Math.min(max, value + 1))}>
        +
      </button>
    </span>
  );
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  label,
}: {
  options: { value: T; label: React.ReactNode; title?: string }[];
  value: T;
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <span className="segmented" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          role="radio"
          aria-checked={o.value === value}
          className={o.value === value ? "on" : ""}
          title={o.title}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </span>
  );
}

const PERCENT = new Intl.NumberFormat("de-DE", { style: "percent", minimumFractionDigits: 1, maximumFractionDigits: 1 });
const INTEGER = new Intl.NumberFormat("de-DE");

export const format = {
  percent: (p: number) => PERCENT.format(p),
  int: (n: number) => INTEGER.format(n),
};
