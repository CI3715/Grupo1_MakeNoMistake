'use client';

import React from 'react';
import type { ContextoSocial, FalloSocial } from '../../types/social';
import { temaSocial } from '../../utils/temaSocial';
import { BotonAccion } from './BotonAccion';

interface Props {
  ctx: ContextoSocial;
  fallo: FalloSocial;
  onReintentar: () => void;
}

/**
 * Pantalla completa cuando una carga falla. Solo Amigos y Grupos avisan: lo personal
 * (cuentas, movimientos, tasas, patrimonio) no depende de la sesión ni de la red.
 */
export function AvisoFalloSocial({ ctx, fallo, onReintentar }: Props) {
  const tema = temaSocial(ctx.modoOscuro);

  const contenido = (() => {
    switch (fallo.tipo) {
      case 'sin_sesion':
        return {
          titulo: 'Inicia sesión para compartir gastos',
          texto:
            'Amigos y Grupos necesitan tu usuario para mostrarte a tu gente y tus grupos. Tus cuentas, movimientos, tasas y patrimonio siguen funcionando sin iniciar sesión.',
          boton: 'Iniciar sesión',
          accion: ctx.irAInicioSesion,
          rol: 'status' as const,
        };
      case 'vencida':
        return {
          titulo: 'Tu sesión venció',
          texto: `${fallo.mensaje} Mientras tanto puedes seguir usando el resto de la aplicación.`,
          boton: 'Iniciar sesión',
          accion: ctx.irAInicioSesion,
          rol: 'alert' as const,
        };
      case 'sin_conexion':
        return {
          titulo: 'Sin conexión con el servidor',
          texto: fallo.mensaje,
          boton: 'Reintentar',
          accion: onReintentar,
          rol: 'alert' as const,
        };
      default:
        return {
          titulo: 'No se pudo cargar',
          texto: fallo.mensaje,
          boton: 'Reintentar',
          accion: onReintentar,
          rol: 'alert' as const,
        };
    }
  })();

  return (
    <div
      role={contenido.rol}
      className={`mx-auto mt-6 max-w-md rounded-2xl border p-7 text-center shadow-sm ${tema.tarjeta}`}
    >
      <h2 className="text-lg font-bold tracking-tight">{contenido.titulo}</h2>
      <p className={`mb-5 mt-2 text-xs ${tema.textoSuave}`}>{contenido.texto}</p>
      <BotonAccion modoOscuro={ctx.modoOscuro} onClick={contenido.accion}>
        {contenido.boton}
      </BotonAccion>
    </div>
  );
}
