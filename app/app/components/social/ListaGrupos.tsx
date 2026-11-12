'use client';

import React from 'react';
import type { ContextoSocial } from '../../types/social';
import { plural } from '../../utils/formatoSocial';
import type { GrupoResumenDto } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { EstadoVacioSocial } from './EstadoVacioSocial';
import { Tarjeta } from './Tarjeta';

interface Props {
  ctx: ContextoSocial;
  grupos: GrupoResumenDto[];
  onAbrir: (grupoId: string) => void;
}

/** Mis grupos. Cada fila es un botón que abre el detalle (miembros, gastos y balances). */
export function ListaGrupos({ ctx, grupos, onAbrir }: Props) {
  const tema = temaSocial(ctx.modoOscuro);
  return (
    <Tarjeta
      modoOscuro={ctx.modoOscuro}
      titulo="Tus grupos"
      descripcion="Abre un grupo para ver sus gastos y quién debe a quién."
    >
      {grupos.length === 0 ? (
        <EstadoVacioSocial
          modoOscuro={ctx.modoOscuro}
          titulo="Todavía no estás en ningún grupo"
          texto="Crea uno con el formulario o acepta una invitación."
        />
      ) : (
        <ul>
          {grupos.map((g) => (
            <li key={g.id} className={`border-b last:border-b-0 ${tema.linea}`}>
              <button
                type="button"
                onClick={() => onAbrir(g.id)}
                className={`flex w-full items-center justify-between gap-3 rounded-lg py-3 text-left cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
                  ctx.modoOscuro ? 'hover:bg-white/5' : 'hover:bg-slate-50'
                }`}
              >
                <span className="min-w-0">
                  <span className="block truncate text-sm font-semibold">{g.nombre}</span>
                  <span className={`block truncate text-[11px] ${tema.textoSuave}`}>
                    {plural(g.cantidad_miembros, 'miembro', 'miembros')} · {g.moneda} ·{' '}
                    {g.rol === 'administrador' ? 'Administrador' : 'Miembro'}
                    {g.actividad ? ` · ${g.actividad}` : ''}
                  </span>
                </span>
                <span className={`shrink-0 text-xs font-semibold ${tema.textoEnlace}`}>
                  Abrir <span aria-hidden="true">→</span>
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <p className={`mt-3 text-[11px] ${tema.textoSuave}`}>
        Cada grupo lleva su propia moneda; los balances de un grupo nunca se mezclan con los de otro.
      </p>
    </Tarjeta>
  );
}
