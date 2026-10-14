import type { PropsWithChildren, SelectHTMLAttributes } from "react";

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  hint?: string;
}

export function Select({ label, hint, children, ...props }: PropsWithChildren<SelectProps>) {
  const control = <select {...props}>{children}</select>;
  if (!label && !hint) return control;
  return <label className="form-field">{label && <span>{label}</span>}{control}{hint && <small>{hint}</small>}</label>;
}
