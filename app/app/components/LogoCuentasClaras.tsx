import React from 'react';

interface Props {
  /** «claro»: texto blanco (sobre fondos morados/oscuros). «oscuro»: texto oscuro (sobre fondos claros). */
  tono?: 'claro' | 'oscuro';
  /** Muestra «TU DINERO, EN ORDEN» debajo del nombre. */
  mostrarLema?: boolean;
  className?: string;
}

/**
 * Logo de Cuentas Claras: ícono de barras + nombre. Mismo layout del Figma
 * (fila, centrado, gap 10px, alto 36px). Se usa en la barra lateral y en la bienvenida.
 */
export const LogoCuentasClaras: React.FC<Props> = ({
  tono = 'claro',
  mostrarLema = true,
  className = '',
}) => {
  const colorNombre = tono === 'claro' ? 'text-white' : 'text-[#211337]';
  const colorLema = tono === 'claro' ? 'text-purple-300/70' : 'text-gray-500';

  return (
    <div className={`flex flex-row items-center gap-[10px] h-9 ${className}`} role="img" aria-label="Cuentas claras">
      <svg width="36" height="36" viewBox="0 0 36 36" fill="none" aria-hidden="true" className="shrink-0">
        <rect x="0.5" y="0.5" width="35" height="35" rx="9.5" fill="#2A1A47" stroke="#4B2F7A" />
        <rect x="9" y="19" width="4.5" height="8" rx="1.2" fill="#C4B5FD" />
        <rect x="15.75" y="13" width="4.5" height="14" rx="1.2" fill="#A78BFA" />
        <rect x="22.5" y="8" width="4.5" height="19" rx="1.2" fill="#9333EA" />
      </svg>
      <div className="flex flex-col justify-center leading-none">
        <span className={`text-[15px] font-semibold tracking-tight lowercase ${colorNombre}`}>
          cuentas claras
        </span>
        {mostrarLema && (
          <span className={`mt-1 text-[8px] font-medium tracking-[0.14em] uppercase ${colorLema}`}>
            Tu dinero, en orden
          </span>
        )}
      </div>
    </div>
  );
};
