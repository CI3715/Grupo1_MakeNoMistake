'use client';

import React from 'react';
import type { ControlSesion } from '../utils/useSesion';
import { SelectorApariencia } from './SelectorApariencia';
import {
  IconoAmigos,
  IconoBilletera,
  IconoCopiaSeguridad,
  IconoDetalleCuenta,
  IconoLogo,
  IconoPatrimonio,
  IconoRecibo,
  IconoSincronizacion,
  IconoTasasCambio,
  IconoUsuario,
} from './Iconos';

export interface SidebarProps {
  pantallaActual: string;
  setPantallaActual: (pantalla: string) => void;
  control: ControlSesion;
  modoOscuro?: boolean;
  toggleModoOscuro?: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  pantallaActual,
  setPantallaActual,
  control,
}) => {
  const sesionVigente = control.sesion.estado === 'vigente';
  const nombreVisible = control.sesion.nombreVisible ?? 'Mi perfil';
  const inicialUsuario = nombreVisible.trim().charAt(0).toUpperCase() || 'U';

  const secciones = [
    {
      titulo: 'Finanzas',
      items: [
        {
          id: 'mis-cuentas',
          etiqueta: 'Mis cuentas',
          icono: <IconoBilletera className="h-4 w-4 shrink-0" />,
        },
        {
          id: 'detalle-cuenta',
          etiqueta: 'Detalle de cuenta',
          icono: <IconoDetalleCuenta className="h-4 w-4 shrink-0" />,
        },
        {
          id: 'tasas',
          etiqueta: 'Tasas de cambio',
          icono: <IconoTasasCambio className="h-4 w-4 shrink-0" />,
        },
        {
          id: 'patrimonio',
          etiqueta: 'Patrimonio',
          icono: <IconoPatrimonio className="h-4 w-4 shrink-0" />,
        },
      ],
    },
    {
      titulo: 'Social y Grupos',
      items: [
        {
          id: 'amigos',
          etiqueta: 'Amigos',
          icono: <IconoAmigos className="h-4 w-4 shrink-0" />,
        },
        {
          id: 'grupos',
          etiqueta: 'Grupos',
          icono: <IconoRecibo className="h-4 w-4 shrink-0" />,
        },
      ],
    },
    {
      titulo: 'Herramientas',
      items: [
        {
          id: 'copia-seguridad',
          etiqueta: 'Copia de seguridad',
          icono: <IconoCopiaSeguridad className="h-4 w-4 shrink-0" />,
        },
        {
          id: 'sincronizacion-libro',
          etiqueta: 'Sincronización',
          icono: <IconoSincronizacion className="h-4 w-4 shrink-0" />,
        },
      ],
    },
  ];

  return (
    <aside className="flex h-screen w-64 shrink-0 flex-col justify-between border-r border-borde bg-superficie p-5 text-texto transition-colors">
      <div className="space-y-6 overflow-y-auto">
        {/* Cabecera de la Barra Lateral */}
        <div className="flex items-center gap-2.5 border-b border-borde pb-4">
          <span className="grid size-9 place-items-center rounded-xl bg-acento text-acento-fg shadow-xs">
            <IconoLogo width={20} height={20} />
          </span>
          <div>
            <span className="text-sm font-bold tracking-tight text-texto">Cuentas Claras</span>
            <span className="ml-1.5 rounded-md bg-superficie-2 px-1.5 py-0.5 text-[9px] font-bold tracking-wider text-suave">
              LOCAL-FIRST
            </span>
          </div>
        </div>

        {/* Grupos de navegación */}
        <nav aria-label="Navegación principal" className="space-y-5">
          {secciones.map((sec) => (
            <div key={sec.titulo} className="space-y-1">
              <p className="px-2.5 text-[11px] font-bold uppercase tracking-wider text-suave">
                {sec.titulo}
              </p>
              <div className="space-y-0.5">
                {sec.items.map((item) => {
                  const activo = pantallaActual === item.id;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setPantallaActual(item.id)}
                      className={`flex w-full cursor-pointer items-center gap-2.5 rounded-xl px-3 py-2 text-left text-xs font-semibold transition ${
                        activo
                          ? 'bg-acento text-acento-fg shadow-xs'
                          : 'text-suave hover:bg-superficie-2 hover:text-texto'
                      }`}
                    >
                      {item.icono}
                      <span>{item.etiqueta}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </nav>
      </div>

      {/* Pie de la Barra Lateral: Usuario y Apariencia */}
      <div className="border-t border-borde pt-4 space-y-2.5">
        <div className="flex items-center justify-between">
          {sesionVigente ? (
            <button
              type="button"
              onClick={() => setPantallaActual('perfil')}
              className={`flex flex-1 cursor-pointer items-center gap-2 rounded-xl p-2 text-left text-xs font-semibold transition ${
                pantallaActual === 'perfil'
                  ? 'bg-superficie-2 text-texto'
                  : 'text-suave hover:bg-superficie-2 hover:text-texto'
              }`}
            >
              <span className="flex size-7 items-center justify-center rounded-full bg-acento text-xs font-bold text-acento-fg">
                {inicialUsuario}
              </span>
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-bold text-texto">{nombreVisible}</p>
                <p className="text-[10px] text-suave">Sesión activa</p>
              </div>
            </button>
          ) : (
            <button
              type="button"
              onClick={() => setPantallaActual('inicio-sesion')}
              className={`flex flex-1 cursor-pointer items-center gap-2 rounded-xl p-2 text-left text-xs font-semibold transition ${
                pantallaActual === 'inicio-sesion' || pantallaActual === 'registro'
                  ? 'bg-superficie-2 text-texto'
                  : 'text-suave hover:bg-superficie-2 hover:text-texto'
              }`}
            >
              <span className="grid size-7 place-items-center rounded-full bg-superficie-2 text-suave">
                <IconoUsuario className="h-3.5 w-3.5" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-xs font-bold text-texto">Iniciar sesión</p>
                <p className="text-[10px] text-suave">Amigos y grupos</p>
              </div>
            </button>
          )}

          <div className="shrink-0 pl-1">
            <SelectorApariencia />
          </div>
        </div>
      </div>
    </aside>
  );
};
