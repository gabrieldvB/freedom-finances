import { CalendarDays, ChevronLeft, ChevronRight } from "lucide-react";
import { formatMonth, shiftMonth } from "../../lib/dates";

export function MonthPicker({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return <div className="month-picker" aria-label="Navegação do mês fiscal">
    <button className="icon-button" onClick={() => onChange(shiftMonth(value, -1))} aria-label="Mês anterior"><ChevronLeft size={17} /></button>
    <span className="month-picker-value"><CalendarDays size={15} /><strong>{formatMonth(value)}</strong></span>
    <button className="icon-button" onClick={() => onChange(shiftMonth(value, 1))} aria-label="Próximo mês"><ChevronRight size={17} /></button>
  </div>;
}
