'use client';

import React, { useEffect, useRef } from 'react';
import { temaSocial } from '../../utils/temaSocial';

/* Piezas pequeñas y reutilizables de las pantallas de Amigos y Grupos. Solo presentación. */

// ---------- Estructura ----------

interface TarjetaProps {
  modoOscuro: boolean;
  titulo?: string;
  /** Texto pequeño a la derecha del título (por ejemplo, una cuenta). */
  accesorio?: React.ReactNode;
  descripcion?: string;
  className?: string;
  children: React.ReactNode;
}

export const TarjetaSocial: React.FC<TarjetaProps> = ({
  modoOscuro,
  titulo,
  accesorio,
  descripcion,
  className = '',
  children,
}) => {
  const tema = temaSocial(modoOscuro);
  return (
    <section className={`min-w-0 rounded-2xl border p-5 ${tema.tarjeta} ${className}`}>
      {(titulo || accesorio) && (
        <div className="mb-1 flex items-center justify-between gap-3">
          {titulo && <h2 className="text-sm font-semibold">{titulo}</h2>}
          {accesorio}
        </div>
      )}
      {descripcion && <p className={`mb-4 text-[11px] ${tema.textoSuave}`}>{descripcion}</p>}
      {!descripcion && titulo && <div className="mb-3" />}
      {children}
    </section>
  );
};

interface EncabezadoProps {
  modoOscuro: boolean;
  titulo: string;
  subtitulo?: string;
  /** Botón de acción principal a la derecha. */
  accion?: React.ReactNode;
  volver?: { texto: string; onClick: () => void };
  /** Lleva el foco al título al abrir (para subpantallas: lector de pantalla y teclado no se pierden). */
  enfocar?: boolean;
}

export const EncabezadoSocial: React.FC<EncabezadoProps> = ({
  modoOscuro,
  titulo,
  subtitulo,
  accion,
  volver,
  enfocar = false,
}) => {
  const tema = temaSocial(modoOscuro);
  const refTitulo = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (enfocar) refTitulo.current?.focus();
  }, [enfocar]);
  return (
    <header className="mb-5">
      {volver && (
        <button
          type="button"
          onClick={volver.onClick}
          className={`mb-2 text-xs font-semibold cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 rounded ${tema.textoEnlace}`}
        >
          ← {volver.texto}
        </button>
      )}
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h1
            ref={refTitulo}
            tabIndex={-1}
            className={`break-words text-2xl font-bold focus:outline-none ${tema.titulo}`}
          >
            {titulo}
          </h1>
          {subtitulo && <p className={`mt-1 text-xs ${tema.textoSuave}`}>{subtitulo}</p>}
        </div>
        {accion}
      </div>
    </header>
  );
};

/** Cifra destacada con su título (como las tarjetas de arriba del diseño). */
export const TarjetaCifra: React.FC<{
  modoOscuro: boolean;
  titulo: string;
  cifra: string;
  nota?: string;
  destacada?: boolean;
}> = ({ modoOscuro, titulo, cifra, nota, destacada = false }) => {
  const tema = temaSocial(modoOscuro);
  return (
    <div
      className={`rounded-2xl border p-4 ${
        destacada ? 'border-transparent bg-[#7C3AED] text-white' : tema.tarjeta
      }`}
    >
      <p className={`text-[11px] ${destacada ? 'text-purple-100' : tema.textoSuave}`}>{titulo}</p>
      <p className="mt-1 text-xl font-bold">{cifra}</p>
      {nota && (
        <p className={`mt-1 text-[11px] ${destacada ? 'text-purple-100' : tema.textoSuave}`}>{nota}</p>
      )}
    </div>
  );
};

// ---------- Avisos y estados ----------

interface AvisoProps {
  modoOscuro: boolean;
  tono: 'error' | 'ambar' | 'exito' | 'info';
  children: React.ReactNode;
  /** Botón de acción dentro del aviso (por ejemplo «Reintentar»). */
  accion?: { texto: string; onClick: () => void };
  className?: string;
}

/** Aviso en línea. Los errores y avisos usan role="alert"; los de éxito, role="status". */
export const AvisoSocial: React.FC<AvisoProps> = ({ modoOscuro, tono, children, accion, className = '' }) => {
  const tema = temaSocial(modoOscuro);
  const colores = {
    error: tema.avisoRojo,
    ambar: tema.avisoAmbar,
    exito: tema.avisoVerde,
    info: tema.avisoMorado,
  }[tono];
  return (
    <div
      role={tono === 'exito' || tono === 'info' ? 'status' : 'alert'}
      className={`flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl border px-4 py-3 text-xs ${colores} ${className}`}
    >
      <p className="min-w-0 flex-1 font-medium">{children}</p>
      {accion && (
        <button
          type="button"
          onClick={accion.onClick}
          className="rounded-lg bg-[#9333EA] px-3 py-1.5 font-semibold text-white hover:bg-purple-600 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400"
        >
          {accion.texto}
        </button>
      )}
    </div>
  );
};

/** Esqueleto de carga: ocupa el lugar de una tarjeta mientras llegan los datos. */
export const CargandoSocial: React.FC<{ modoOscuro: boolean; filas?: number }> = ({ modoOscuro, filas = 3 }) => {
  const tema = temaSocial(modoOscuro);
  return (
    <div role="status" aria-busy="true" aria-label="Cargando" className="space-y-3">
      {Array.from({ length: filas }, (_, i) => (
        <div key={i} className={`h-16 animate-pulse motion-reduce:animate-none rounded-2xl border ${tema.tarjeta}`} />
      ))}
    </div>
  );
};

export const VacioSocial: React.FC<{ modoOscuro: boolean; children: React.ReactNode }> = ({ modoOscuro, children }) => {
  const tema = temaSocial(modoOscuro);
  return <p className={`rounded-xl border border-dashed px-4 py-5 text-center text-xs ${tema.separador} ${tema.textoSuave}`}>{children}</p>;
};

export const ChipSocial: React.FC<{
  modoOscuro: boolean;
  tono?: 'morado' | 'ambar' | 'verde';
  children: React.ReactNode;
}> = ({ modoOscuro, tono = 'morado', children }) => {
  const tema = temaSocial(modoOscuro);
  const color = tono === 'ambar' ? tema.chipAmbar : tono === 'verde' ? tema.chipVerde : tema.chip;
  return (
    <span className={`inline-block max-w-full rounded-full px-2 py-0.5 text-[10px] font-semibold ${color}`}>
      {children}
    </span>
  );
};

// ---------- Botones y selects ----------

interface BotonProps {
  modoOscuro: boolean;
  variante?: 'primario' | 'secundario' | 'peligro' | 'texto';
  type?: 'button' | 'submit';
  onClick?: () => void;
  cargando?: boolean;
  deshabilitado?: boolean;
  ancho?: boolean;
  ariaLabel?: string;
  /** Pone el foco en el botón al aparecer (por ejemplo, en una confirmación). */
  autoEnfocar?: boolean;
  children: React.ReactNode;
}

export const BotonSocial: React.FC<BotonProps> = ({
  modoOscuro,
  variante = 'primario',
  type = 'button',
  onClick,
  cargando = false,
  deshabilitado = false,
  ancho = false,
  ariaLabel,
  autoEnfocar = false,
  children,
}) => {
  const tema = temaSocial(modoOscuro);
  const estilos = {
    primario: 'border-transparent bg-[#9333EA] text-white shadow-md hover:bg-purple-600',
    secundario: `bg-transparent ${tema.botonSecundario}`,
    peligro: modoOscuro
      ? 'border-rose-800/70 bg-rose-950/40 text-rose-200 hover:bg-rose-950/70'
      : 'border-rose-200 bg-rose-50 text-rose-700 hover:bg-rose-100',
    texto: `border-transparent bg-transparent ${tema.textoEnlace}`,
  }[variante];
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={deshabilitado || cargando}
      aria-busy={cargando || undefined}
      aria-label={ariaLabel}
      autoFocus={autoEnfocar}
      className={`inline-flex items-center justify-center gap-2 rounded-xl border px-3.5 py-2 text-xs font-semibold transition cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${estilos} ${
        ancho ? 'w-full' : ''
      }`}
    >
      {cargando && (
        <span
          className="h-3.5 w-3.5 animate-spin motion-reduce:animate-none rounded-full border-2 border-current/40 border-t-current"
          aria-hidden="true"
        />
      )}
      {children}
    </button>
  );
};

interface SelectProps {
  id: string;
  etiqueta: string;
  valor: string;
  onCambio: (valor: string) => void;
  opciones: { valor: string; texto: string }[];
  error?: string;
  deshabilitado?: boolean;
  modoOscuro: boolean;
}

/** Select con el mismo aspecto y el mismo error debajo que `CampoTexto`. */
export const SelectSocial: React.FC<SelectProps> = ({
  id,
  etiqueta,
  valor,
  onCambio,
  opciones,
  error,
  deshabilitado = false,
  modoOscuro,
}) => {
  const tema = temaSocial(modoOscuro);
  return (
    <div>
      <label htmlFor={id} className={`block text-xs font-semibold mb-1 ${tema.textoSuave}`}>
        {etiqueta}
      </label>
      <select
        id={id}
        name={id}
        value={valor}
        onChange={(e) => onCambio(e.target.value)}
        disabled={deshabilitado}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? `${id}-error` : undefined}
        style={error ? { ...tema.estiloInput, borderColor: '#f43f5e' } : tema.estiloInput}
        className="w-full border px-3 py-2.5 rounded-xl text-xs focus:outline-none focus:ring-2 focus:ring-purple-500 disabled:opacity-60"
      >
        {opciones.map((o) => (
          <option key={o.valor} value={o.valor}>
            {o.texto}
          </option>
        ))}
      </select>
      {error && (
        <p id={`${id}-error`} role="alert" className="mt-1 text-[11px] font-medium text-rose-400">
          {error}
        </p>
      )}
    </div>
  );
};
