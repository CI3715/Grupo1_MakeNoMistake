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
}

export const Sidebar: React.FC<SidebarProps> = ({
  pantallaActual,
  setPantallaActual,
  control,
}) => {
  const sesionVigente = control.sesion.estado === 'vigente';
  const nombreVisible = control.sesion.nombreVisible ?? 'Mi perfil';
  const inicialUsuario = nombreVisible.trim().charAt(0).toUpperCase() || 'U';

  const items = [
    {
      id: 'mis-cuentas',
      etiqueta: 'Mis cuentas',
      icono: <IconoBilletera className="h-5 w-5 shrink-0" />,
    },
    {
      id: 'detalle-cuenta',
      etiqueta: 'Detalle de cuenta',
      icono: <IconoDetalleCuenta className="h-5 w-5 shrink-0" />,
    },
    {
      id: 'tasas',
      etiqueta: 'Tasas de cambio',
      icono: <IconoTasasCambio className="h-5 w-5 shrink-0" />,
    },
    {
      id: 'patrimonio',
      etiqueta: 'Patrimonio',
      icono: <IconoPatrimonio className="h-5 w-5 shrink-0" />,
    },
    {
      id: 'amigos',
      etiqueta: 'Amigos',
      icono: <IconoAmigos className="h-5 w-5 shrink-0" />,
    },
    {
      id: 'grupos',
      etiqueta: 'Grupos',
      icono: <IconoRecibo className="h-5 w-5 shrink-0" />,
    },
    {
      id: 'copia-seguridad',
      etiqueta: 'Copia de seguridad',
      icono: <IconoCopiaSeguridad className="h-5 w-5 shrink-0" />,
    },
    {
      id: 'sincronizacion-libro',
      etiqueta: 'Sincronización',
      icono: <IconoSincronizacion className="h-5 w-5 shrink-0" />,
    },
  ];

  return (
    <aside
      aria-label="Barra lateral de navegación"
      className="group/sidebar relative my-4 ml-4 flex h-[calc(100vh-2rem)] w-20 flex-col justify-between rounded-[2.25rem] border border-borde bg-superficie p-3.5 text-texto shadow-lg transition-all duration-300 ease-in-out hover:w-64 hover:shadow-2xl z-40 shrink-0"
    >
      {/* Zona Superior: Logo / Marca */}
      <div className="flex flex-col items-center">
        <button
          type="button"
          onClick={() => setPantallaActual('mis-cuentas')}
          title="Cuentas Claras"
          className="flex h-12 w-full cursor-pointer items-center gap-3 overflow-hidden rounded-2xl px-2 transition hover:bg-superficie-2"
        >
          <span className="grid size-11 shrink-0 place-items-center rounded-2xl bg-acento text-acento-fg shadow-xs">
            <IconoLogo width={22} height={22} />
          </span>
          <div className="hidden min-w-0 opacity-0 transition-opacity duration-200 group-hover/sidebar:block group-hover/sidebar:opacity-100">
            <p className="truncate text-sm font-bold tracking-tight text-texto">Cuentas Claras</p>
            <p className="text-[10px] font-bold tracking-wider text-acento-texto uppercase">
              LOCAL-FIRST
            </p>
          </div>
        </button>

        {/* Separador fino */}
        <div className="my-3 w-8 border-b border-borde transition-all group-hover/sidebar:w-full" />

        {/* Lista de Iconos de Navegación */}
        <nav aria-label="Menú principal" className="w-full space-y-2">
          {items.map((item) => {
            const activo = pantallaActual === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setPantallaActual(item.id)}
                title={item.etiqueta}
                className={`relative flex h-12 w-full cursor-pointer items-center overflow-hidden rounded-2xl px-2.5 transition-all duration-200 ${
                  activo
                    ? 'bg-acento text-acento-fg shadow-sm'
                    : 'text-suave hover:bg-superficie-2 hover:text-texto'
                }`}
              >
                <span className="grid size-7 shrink-0 place-items-center">
                  {item.icono}
                </span>
                <span className="hidden whitespace-nowrap pl-3 text-sm font-semibold opacity-0 transition-opacity duration-200 group-hover/sidebar:inline-block group-hover/sidebar:opacity-100">
                  {item.etiqueta}
                </span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Zona Inferior: Personalización y Perfil */}
      <div className="flex flex-col items-center gap-2 border-t border-borde pt-3 w-full">
        <div className="w-full flex justify-center group-hover/sidebar:justify-start">
          <SelectorApariencia />
        </div>

        {sesionVigente ? (
          <button
            type="button"
            onClick={() => setPantallaActual('perfil')}
            title={`Perfil: ${nombreVisible}`}
            className={`flex h-12 w-full cursor-pointer items-center overflow-hidden rounded-2xl px-2 transition ${
              pantallaActual === 'perfil'
                ? 'bg-acento-suave text-acento-texto'
                : 'text-suave hover:bg-superficie-2 hover:text-texto'
            }`}
          >
            <span className="flex size-8 shrink-0 items-center justify-center rounded-full bg-acento text-xs font-bold text-acento-fg shadow-xs">
              {inicialUsuario}
            </span>
            <div className="hidden min-w-0 pl-3 opacity-0 transition-opacity duration-200 group-hover/sidebar:block group-hover/sidebar:opacity-100 text-left">
              <p className="truncate text-xs font-bold text-texto">{nombreVisible}</p>
              <p className="text-[10px] text-suave">Sesión activa</p>
            </div>
          </button>
        ) : (
          <button
            type="button"
            onClick={() => setPantallaActual('inicio-sesion')}
            title="Iniciar sesión"
            className="flex h-12 w-full cursor-pointer items-center overflow-hidden rounded-2xl px-2 text-suave transition hover:bg-superficie-2 hover:text-texto"
          >
            <span className="grid size-8 shrink-0 place-items-center rounded-full bg-superficie-2">
              <IconoUsuario className="h-4 w-4" />
            </span>
            <div className="hidden min-w-0 pl-3 opacity-0 transition-opacity duration-200 group-hover/sidebar:block group-hover/sidebar:opacity-100 text-left">
              <p className="text-xs font-bold text-texto">Iniciar sesión</p>
              <p className="text-[10px] text-suave">Amigos y grupos</p>
            </div>
          </button>
        )}
      </div>
    </aside>
  );
};
