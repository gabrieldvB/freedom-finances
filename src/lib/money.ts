const currencyFormatter = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
});

export function formatMoney(cents: number): string {
  return currencyFormatter.format(cents / 100);
}

export function parseMoney(value: string): number | null {
  const cleaned = value.trim().replace(/\s/g, "");
  if (!cleaned) return null;

  const normalized = cleaned
    .replace(/^R\$/i, "")
    .replace(/\./g, "")
    .replace(",", ".");
  if (!/^-?\d+(\.\d{0,2})?$/.test(normalized)) return null;

  const negative = normalized.startsWith("-");
  const unsigned = normalized.replace("-", "");
  const [whole, decimal = ""] = unsigned.split(".");
  const cents = Number.parseInt(whole, 10) * 100 + Number.parseInt(decimal.padEnd(2, "0") || "0", 10);
  return negative ? -cents : cents;
}

export function moneyInput(cents: number): string {
  return (cents / 100).toFixed(2).replace(".", ",");
}
