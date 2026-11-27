'use client';

import React from 'react';
import {
  IconoBilletera,
  IconoCheckCirculo,
  IconoFlechaDerecha,
  IconoLogo,
  IconoUsuario,
} from '../Iconos';
import { SelectorApariencia } from '../SelectorApariencia';

interface Props {
  modoOscuro: boolean;
  onContinuarSinRegistro: () => void;
  onCrearUsuario: () => void;
  onIniciarSesion: () => void;
}

export const PantallaBienvenida: React.FC<Props> = ({
  onContinuarSinRegistro,
  onCrearUsuario,
  onIniciarSesion,
}) => {
  return (
    <div className="fixed inset-0 z-[70] overflow-y-auto bg-fondo text-texto p-4 sm:p-8 flex flex-col justify-between">
      {/* Barra superior con marca y personalización inmediata */}
      <header className="mx-auto w-full max-w-6xl flex items-center justify-between pb-6 sm:pb-8">
        <div className="flex items-center gap-3">
          <span className="grid size-11 place-items-center rounded-2xl bg-acento text-acento-fg shadow-xs">
            <IconoLogo width={22} height={22} />
          </span>
          <div>
            <span className="block text-base font-extrabold tracking-tight text-texto sm:text-lg">
              Cuentas Claras
            </span>
            <span className="block text-[10px] font-bold tracking-widest text-acento-texto uppercase">
              LOCAL-FIRST
            </span>
          </div>
        </div>

        {/* Selector de apariencia y accesibilidad */}
        <div className="flex items-center gap-2">
          <SelectorApariencia variante="superficie" />
        </div>
      </header>

      {/* Contenedor principal de dos columnas con estilo Quixotic / Cuentas Claras */}
      <main className="mx-auto w-full max-w-6xl my-auto grid grid-cols-1 lg:grid-cols-12 gap-6 items-stretch">
        {/* Panel Izquierdo: Presentación y Tarjeta Ilustrativa */}
        <section className="lg:col-span-6 relative overflow-hidden rounded-[2.5rem] bg-linear-to-br from-grad-a to-grad-b p-8 sm:p-12 text-white shadow-xl flex flex-col justify-between min-h-[460px]">
          {/* Destellos decorativos difusos */}
          <div className="pointer-events-none absolute -right-16 -top-16 size-56 rounded-full bg-white/10 blur-3xl" />
          <div className="pointer-events-none absolute -left-16 -bottom-16 size-56 rounded-full bg-white/10 blur-3xl" />

          <div className="relative space-y-5">
            <span className="inline-flex items-center gap-2 rounded-full bg-white/15 px-4 py-1.5 text-xs font-bold uppercase tracking-wider text-white ring-1 ring-white/25 backdrop-blur-md">
              Personal · Multimoneda · Local-First
            </span>

            <h1 className="text-3xl sm:text-4xl font-extrabold leading-tight tracking-tight text-white">
              Tu dinero, claro.
              <br />
              Tus datos, tuyos.
            </h1>

            <p className="max-w-md text-sm text-white/85 leading-relaxed">
              Un asistente financiero diseñado para organizar tus cuentas y compartir gastos, con total
              privacidad y sin depender de servidores para tu libro personal.
            </p>

            {/* Tarjeta ilustrativa con el estilo de la app */}
            <div className="w-full max-w-sm rounded-[2rem] border border-white/20 bg-white/10 p-6 backdrop-blur-md shadow-lg">
              <div className="flex items-center justify-between text-xs font-bold uppercase tracking-widest text-white/80">
                <span className="flex items-center gap-2">
                  <IconoBilletera className="h-4 w-4" />
                  Libro Personal
                </span>
                <span className="rounded-full bg-white/20 px-2 py-0.5 text-[10px] text-white">
                  Local
                </span>
              </div>

              <div className="my-3">
                <span className="text-[11px] text-white/70 font-medium block">Patrimonio consolidado</span>
                <p className="text-3xl font-extrabold tracking-tight tabular-nums">
                  1.980,00 USD
                </p>
              </div>

              <div className="flex items-center justify-between pt-3 border-t border-white/15 text-[11px] text-white/80">
                <span>USD · EUR · VES · BTC</span>
                <span className="font-mono text-[10px] opacity-75">SQLite inmutable</span>
              </div>
            </div>
          </div>

          <div className="relative pt-6 flex items-center gap-6 text-xs text-white/75 border-t border-white/15 mt-6">
            <span>Windows · Linux · Escritorio</span>
            <span>Local-First</span>
          </div>
        </section>

        {/* Panel Derecho: Primer Acceso y Acciones Clave */}
        <section className="lg:col-span-6 rounded-[2.5rem] border border-borde bg-superficie p-8 sm:p-12 text-texto shadow-sm flex flex-col justify-between">
          <div className="space-y-6">
            <div>
              <span className="text-xs font-bold uppercase tracking-wider text-acento-texto">
                Bienvenida a Cuentas Claras
              </span>
              <h2 className="text-3xl font-extrabold tracking-tight text-texto mt-1">
                Empieza por ti.
              </h2>
              <p className="text-sm text-suave mt-2">
                No necesitas crear una cuenta de usuario para gestionar tus finanzas personales.
              </p>
            </div>

            {/* Lista de garantías arquitectónicas del sistema */}
            <ul className="space-y-4">
              <li className="flex items-start gap-3.5">
                <span className="grid size-8 shrink-0 place-items-center rounded-xl bg-acento/10 text-acento-texto">
                  <IconoCheckCirculo width={18} height={18} />
                </span>
                <div>
                  <p className="text-sm font-bold text-texto">Funciona 100% sin conexión</p>
                  <p className="text-xs text-suave">
                    Registra y consulta tus cuentas y movimientos sin requerir internet ni servidor.
                  </p>
                </div>
              </li>

              <li className="flex items-start gap-3.5">
                <span className="grid size-8 shrink-0 place-items-center rounded-xl bg-acento/10 text-acento-texto">
                  <IconoCheckCirculo width={18} height={18} />
                </span>
                <div>
                  <p className="text-sm font-bold text-texto">Privacidad por diseño (INV-16)</p>
                  <p className="text-xs text-suave">
                    Sin telemetría ni conexión a tus bancos. Tus finanzas nunca salen de este equipo.
                  </p>
                </div>
              </li>

              <li className="flex items-start gap-3.5">
                <span className="grid size-8 shrink-0 place-items-center rounded-xl bg-acento/10 text-acento-texto">
                  <IconoCheckCirculo width={18} height={18} />
                </span>
                <div>
                  <p className="text-sm font-bold text-texto">Libro inmutable de eventos</p>
                  <p className="text-xs text-suave">
                    Tu historial contable reside en tu base de datos SQLite local inmutable.
                  </p>
                </div>
              </li>
            </ul>

            {/* Botón principal de acceso directo sin registro */}
            <button
              type="button"
              onClick={onContinuarSinRegistro}
              className="flex w-full cursor-pointer items-center justify-center gap-2 rounded-full bg-acento px-6 py-4 text-sm font-bold text-acento-fg shadow-md transition hover:brightness-110 active:scale-[0.98]"
            >
              <span>Continuar sin registro</span>
              <IconoFlechaDerecha width={16} height={16} />
            </button>

            {/* Bloque de compartir gastos */}
            <div className="rounded-2xl border border-borde bg-superficie-2 p-5 space-y-3">
              <div>
                <p className="text-xs font-bold text-texto uppercase tracking-wider flex items-center gap-1.5">
                  <IconoUsuario className="h-3.5 w-3.5 text-acento-texto" />
                  ¿Quieres compartir gastos?
                </p>
                <p className="text-xs text-suave mt-1 leading-relaxed">
                  Crea una identidad solo para amigos y grupos cuando quieras. Tu libro personal seguirá
                  siendo completamente privado.
                </p>
              </div>

              <div className="flex items-center gap-3 pt-1">
                <button
                  type="button"
                  onClick={onCrearUsuario}
                  className="cursor-pointer rounded-full bg-superficie border border-borde px-4 py-2 text-xs font-bold text-texto shadow-xs transition hover:bg-superficie-hover active:scale-[0.98]"
                >
                  Crear usuario
                </button>
                <button
                  type="button"
                  onClick={onIniciarSesion}
                  className="cursor-pointer text-xs font-bold text-acento-texto hover:underline transition"
                >
                  Iniciar sesión
                </button>
              </div>
            </div>
          </div>

          <p className="text-[11px] text-suave pt-4 border-t border-borde mt-6">
            Cuentas Claras · Aplicación instalada de finanzas personales local-first.
          </p>
        </section>
      </main>

      {/* Pie inferior discreto */}
      <footer className="mx-auto w-full max-w-6xl pt-4 text-center text-[11px] text-suave">
        Tus datos contables pertenecen exclusivamente a tu dispositivo.
      </footer>
    </div>
  );
};
