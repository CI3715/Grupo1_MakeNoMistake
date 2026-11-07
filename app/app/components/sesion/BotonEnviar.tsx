import React from 'react';

interface Props {
  cargando: boolean;
  textoNormal: string;
  textoCargando: string;
}

/** Botón de envío con estado de carga (deshabilitado + indicador). */
export const BotonEnviar: React.FC<Props> = ({ cargando, textoNormal, textoCargando }) => (
  <button
    type="submit"
    disabled={cargando}
    aria-busy={cargando}
    className="w-full flex items-center justify-center gap-2 bg-[#9333EA] hover:bg-purple-600 disabled:opacity-70 disabled:cursor-not-allowed text-white py-2.5 rounded-xl text-xs font-semibold shadow-md transition cursor-pointer"
  >
    {cargando && (
      <span
        className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/40 border-t-white"
        aria-hidden="true"
      />
    )}
    {cargando ? textoCargando : textoNormal}
  </button>
);
