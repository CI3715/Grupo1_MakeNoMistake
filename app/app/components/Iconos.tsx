import type { SVGProps } from 'react';
import type { TipoCuenta } from '../types/cuentas';

/** Iconos de trazo simple (sin dependencias). Heredan el color con `currentColor`. */
function Svg(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={20}
      height={20}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    />
  );
}

export function IconoLogo(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <circle cx="12" cy="12" r="9" />
      <path d="m8 12.5 2.8 2.8 5.2-5.6" />
    </Svg>
  );
}

export function IconoEfectivo(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <rect x="2.5" y="6" width="19" height="12" rx="2.5" />
      <circle cx="12" cy="12" r="2.5" />
      <path d="M6 12h.01M18 12h.01" />
    </Svg>
  );
}

export function IconoBanco(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M12 3 3 8h18l-9-5Z" />
      <path d="M5 10.5v7M9.5 10.5v7M14.5 10.5v7M19 10.5v7M3 21h18" />
    </Svg>
  );
}

export function IconoBilletera(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H18a1 1 0 0 1 1 1v1.5" />
      <rect x="3" y="7.5" width="18" height="12" rx="2.5" />
      <path d="M16.5 13.5h.01" />
    </Svg>
  );
}

export function IconoMas(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M12 5v14M5 12h14" />
    </Svg>
  );
}

export function IconoPaleta(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M12 3a9 9 0 1 0 0 18c1 0 1.7-.8 1.7-1.7 0-.5-.2-.9-.5-1.2-.3-.3-.5-.7-.5-1.2 0-.9.7-1.6 1.6-1.6H16a5 5 0 0 0 5-5c0-4-4-7.3-9-7.3Z" />
      <path d="M7.5 11h.01M10 7.5h.01M14.5 7.5h.01" />
    </Svg>
  );
}

export function IconoTipo({ tipo, ...props }: { tipo: TipoCuenta | string } & SVGProps<SVGSVGElement>) {
  switch (tipo) {
    case 'banco':
      return <IconoBanco {...props} />;
    case 'billetera_digital':
      return <IconoBilletera {...props} />;
    default:
      return <IconoEfectivo {...props} />;
  }
}
