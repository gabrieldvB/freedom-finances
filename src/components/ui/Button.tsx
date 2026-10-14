import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "success" | "warning";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  icon?: ReactNode;
  loading?: boolean;
}

export function Button({ variant = "secondary", icon, loading = false, className = "", children, disabled, ...props }: ButtonProps) {
  return <button className={`btn btn-${variant} ${className}`.trim()} disabled={disabled || loading} {...props}>
    {loading ? <span className="button-spinner" aria-hidden="true" /> : icon}
    <span>{loading ? "Aguarde…" : children}</span>
  </button>;
}
