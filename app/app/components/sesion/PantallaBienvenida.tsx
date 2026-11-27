'use client';

import React from 'react';
import { LogoCuentasClaras } from '../LogoCuentasClaras';

interface Props {
  modoOscuro: boolean;
  onContinuarSinRegistro: () => void;
  onCrearUsuario: () => void;
  onIniciarSesion: () => void;
}

const Icono: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className="shrink-0">
    {children}
  </svg>
);

/**
 * Bienvenida de primer acceso (diseño de Figma «Escritorio · Bienvenida»).
 * Izquierda: presentación (#211337, 660px, padding 64, gap 48). Derecha: «Primer acceso»
 * (padding 64/88, gap 30). Por debajo de 1280px se apila para que no haya scroll horizontal.
 */
export const PantallaBienvenida: React.FC<Props> = ({
  modoOscuro,
  onContinuarSinRegistro,
  onCrearUsuario,
  onIniciarSesion,
}) => {
  const fondoDerecho = modoOscuro ? 'bg-[#0B0F17] text-white' : 'bg-white text-slate-900';
  const textoSuave = modoOscuro ? 'text-gray-400' : 'text-gray-500';
  const textoTitulo = modoOscuro ? 'text-white' : 'text-[#1a1030]';
  const cajaCompartir = modoOscuro ? 'bg-[#151C28]' : 'bg-[#F6F5FA]';

  return (
    <div className="fixed inset-0 z-[70] overflow-y-auto overflow-x-hidden bg-[#211337]">
      <div className="flex min-h-full flex-col xl:flex-row">
        {/* ---------- Presentación ---------- */}
        <section className="flex flex-col items-start gap-8 xl:gap-12 bg-[#211337] p-8 xl:p-16 xl:w-[660px] xl:shrink-0 text-white">
          <LogoCuentasClaras tono="claro" />

          <div className="flex flex-col items-start gap-6 xl:my-auto">
            <span className="rounded-full bg-[#9333EA]/30 border border-purple-400/30 px-3 py-1 text-[10px] font-semibold tracking-[0.12em] text-purple-200 uppercase">
              Personal · Multimoneda · Local
            </span>

            <h1 className="text-4xl xl:text-[44px] font-semibold leading-[1.15] tracking-tight">
              Tu dinero, claro.
              <br />
              Tus datos, tuyos.
            </h1>

            <p className="max-w-md text-sm text-purple-200/80">
              Un lugar para organizar tus cuentas y compartir gastos, sin perder el control de tu información.
            </p>

            {/* Tarjeta ilustrativa: datos de ejemplo, no son del libro de nadie */}
            <div className="w-full max-w-md rounded-2xl border border-purple-400/20 bg-white/5 p-6">
              <div className="flex items-center justify-between text-[10px] font-semibold tracking-[0.12em] text-purple-200/70 uppercase">
                <span>Mi libro personal</span>
                <Icono>
                  <rect x="5" y="11" width="14" height="10" rx="2" />
                  <path d="M8 11V8a4 4 0 0 1 8 0v3" />
                </Icono>
              </div>
              <p className="mt-3 text-3xl font-semibold tracking-tight">1.980,00 USD</p>
              <p className="mt-1 text-xs text-purple-200/70">Todo tu patrimonio, en una sola mirada.</p>
              <div className="mt-4 flex h-1.5 w-full gap-1.5" aria-hidden="true">
                <span className="h-full flex-[6] rounded-full bg-purple-200" />
                <span className="h-full flex-[1.2] rounded-full bg-purple-400" />
                <span className="h-full flex-[2.5] rounded-full bg-[#9333EA]" />
              </div>
              <p className="mt-3 text-[10px] text-purple-200/60">USD · VES · USDT / Datos ilustrativos</p>
            </div>
          </div>

          <div className="flex items-center gap-6 text-[11px] text-purple-200/70 xl:mt-auto">
            <span className="flex items-center gap-2">
              <Icono>
                <rect x="3" y="4" width="18" height="12" rx="2" />
                <path d="M8 20h8M12 16v4" />
              </Icono>
              Windows · Linux
            </span>
            <span className="flex items-center gap-2">
              <Icono>
                <rect x="7" y="2" width="10" height="20" rx="2" />
                <path d="M11 18h2" />
              </Icono>
              Android
            </span>
          </div>
        </section>

        {/* ---------- Primer acceso ---------- */}
        <section
          className={`flex flex-1 flex-col justify-center items-start gap-[30px] p-8 xl:px-[88px] xl:py-16 ${fondoDerecho}`}
        >
          <div className="w-full max-w-xl flex flex-col gap-[30px]">
            <div>
              <p className={`text-[10px] font-semibold tracking-[0.14em] uppercase ${textoSuave}`}>
                Bienvenida a Cuentas Claras
              </p>
              <h2 className={`mt-3 text-3xl font-semibold tracking-tight ${textoTitulo}`}>Empieza por ti.</h2>
              <p className={`mt-3 text-sm ${textoSuave}`}>
                No necesitas una cuenta de usuario para llevar tus finanzas personales.
              </p>
            </div>

            <ul className="flex flex-col gap-4">
              <li className="flex items-start gap-3">
                <span className="mt-0.5 text-[#9333EA]">
                  <Icono>
                    <path d="M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0M12 19.5h.01M3 3l18 18" />
                  </Icono>
                </span>
                <div>
                  <p className="text-sm font-semibold">Funciona sin conexión</p>
                  <p className={`text-xs ${textoSuave}`}>Registra tus movimientos, estés donde estés.</p>
                </div>
              </li>
              <li className="flex items-start gap-3">
                <span className="mt-0.5 text-[#9333EA]">
                  <Icono>
                    <path d="M12 3l8 3v6c0 4.5-3.2 8-8 9-4.8-1-8-4.5-8-9V6l8-3z" />
                    <path d="M9 12l2 2 4-4" />
                  </Icono>
                </span>
                <div>
                  <p className="text-sm font-semibold">Privacidad por diseño</p>
                  <p className={`text-xs ${textoSuave}`}>Sin telemetría ni conexión a tus bancos.</p>
                </div>
              </li>
              <li className="flex items-start gap-3">
                <span className="mt-0.5 text-[#9333EA]">
                  <Icono>
                    <rect x="3" y="4" width="18" height="12" rx="2" />
                    <path d="M8 20h8M12 16v4" />
                  </Icono>
                </span>
                <div>
                  <p className="text-sm font-semibold">En tu dispositivo</p>
                  <p className={`text-xs ${textoSuave}`}>Tu libro personal permanece contigo.</p>
                </div>
              </li>
            </ul>

            <button
              type="button"
              onClick={onContinuarSinRegistro}
              className="flex w-full items-center justify-center gap-2 rounded-xl bg-[#6D3FD0] hover:bg-[#5b32b5] px-5 py-3 text-sm font-semibold text-white shadow-md transition cursor-pointer"
            >
              <span aria-hidden="true">→</span> Continuar sin registro
            </button>

            <div className={`rounded-2xl p-5 ${cajaCompartir}`}>
              <p className="text-sm font-semibold">¿Quieres compartir gastos?</p>
              <p className={`mt-1 text-xs ${textoSuave}`}>
                Crea una identidad solo para amigos, solicitudes y grupos. Tu libro seguirá siendo privado.
              </p>
              <div className="mt-4 flex items-center gap-4">
                <button
                  type="button"
                  onClick={onCrearUsuario}
                  className="rounded-lg bg-purple-100 px-4 py-2 text-xs font-semibold text-[#6D3FD0] hover:bg-purple-200 transition cursor-pointer"
                >
                  Crear usuario
                </button>
                <button
                  type="button"
                  onClick={onIniciarSesion}
                  className="text-xs font-semibold text-[#6D3FD0] hover:underline cursor-pointer"
                >
                  Iniciar sesión
                </button>
              </div>
            </div>

            <p className={`text-[11px] ${textoSuave}`}>
              Aplicación instalada · No mueve dinero ni procesa pagos.
            </p>
          </div>
        </section>
      </div>
    </div>
  );
};
