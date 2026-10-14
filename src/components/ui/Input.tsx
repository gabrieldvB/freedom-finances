import type { InputHTMLAttributes, ReactNode } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  leadingIcon?: ReactNode;
}

export function Input({ label, hint, leadingIcon, className = "", ...props }: InputProps) {
  const control = <span className={`input-shell ${leadingIcon ? "has-icon" : ""}`}>
    {leadingIcon && <span className="input-icon">{leadingIcon}</span>}
    <input className={className} {...props} />
  </span>;
  if (!label && !hint) return control;
  return <label className="form-field">{label && <span>{label}</span>}{control}{hint && <small>{hint}</small>}</label>;
}
