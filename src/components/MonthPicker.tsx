import { ChevronLeft, ChevronRight } from "lucide-react";
import { formatMonth, shiftMonth } from "../lib/dates";

export function MonthPicker({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <div className="month-picker">
      <button className="icon-button" onClick={() => onChange(shiftMonth(value, -1))} aria-label="Mês anterior"><ChevronLeft size={18} /></button>
      <strong>{formatMonth(value)}</strong>
      <button className="icon-button" onClick={() => onChange(shiftMonth(value, 1))} aria-label="Próximo mês"><ChevronRight size={18} /></button>
    </div>
  );
}
