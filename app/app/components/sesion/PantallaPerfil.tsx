'use client';

import React, { useEffect, useState } from 'react';
import { obtenerPerfilCommand, type UsuarioDto } from '../../utils/tauriCommands';
import { primerMensaje } from '../../utils/erroresSesion';
import { temaSesion } from '../../utils/temaSesion';
import type { ControlSesion } from '../../utils/useSesion';

interface Props {
  modoOscuro: boolean;
  control: ControlSesion;
  setPantallaActual: (pantalla: string) => void;
}

type EstadoPerfil =
  | { tipo: 'cargando' }
  | { tipo: 'listo'; usuario: UsuarioDto }
  | { tipo: 'error'; mensaje: string };

/** Solo formato de fecha para mostrar (convención local, RNF-20). */
function formatearFecha(iso: string | null | undefined, conHora: boolean): string {
  if (!iso) return '—';
  const fecha = new Date(iso);
  if (Number.isNaN(fecha.getTime())) return '—';
  return fecha.toLocaleString('es', conHora ? { dateStyle: 'long', timeStyle: 'short' } : { dateStyle: 'long' });
}

/** Vista simple del perfil: GET /usuarios/yo con el token que guarda el núcleo. */
export const PantallaPerfil: React.FC<Props> = ({ modoOscuro, control, setPantallaActual }) => {
  const tema = temaSesion(modoOscuro);
  const [estado, setEstado] = useState<EstadoPerfil>({ tipo: 'cargando' });
  const [intento, setIntento] = useState(0);
  const [cerrando, setCerrando] = useState(false);
  const [errorCierre, setErrorCierre] = useState<string | null>(null);

  const { refrescar } = control;
  const estadoSesion = control.sesion.estado;

  useEffect(() => {
    if (estadoSesion !== 'vigente') return;
    let activo = true;
    obtenerPerfilCommand()
      .then((usuario) => {
        if (activo) setEstado({ tipo: 'listo', usuario });
      })
      .catch((err: unknown) => {
        if (!activo) return;
        setEstado({ tipo: 'error', mensaje: primerMensaje(err) });
        // Si el servidor respondió 401, el núcleo marcó la sesión como vencida: se vuelve a leer.
        void refrescar();
      });
    return () => {
      activo = false;
    };
  }, [estadoSesion, intento, refrescar]);

  const reintentar = () => {
    setEstado({ tipo: 'cargando' });
    setIntento((n) => n + 1);
  };

  const cerrar = async () => {
    if (cerrando) return;
    setCerrando(true);
    setErrorCierre(null);
    const error = await control.cerrarSesion();
    setCerrando(false);
    if (error) setErrorCierre(error);
    else setPantallaActual('mis-cuentas');
  };

  const encabezado = (
    <div className="mb-5">
      <h2 className="text-2xl font-bold tracking-tight">Mi perfil</h2>
      <p className={`mt-1 text-xs ${tema.textoSuave}`}>
        Tu identidad en el servidor. Tus cuentas y movimientos no salen de tu dispositivo.
      </p>
    </div>
  );

  const botonPrimario =
    'rounded-xl bg-[#9333EA] px-4 py-2 text-xs font-semibold text-white hover:bg-purple-600 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400';
  const botonSecundario = `rounded-xl border px-4 py-2 text-xs font-semibold cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
    modoOscuro ? 'border-gray-700 hover:bg-slate-800' : 'border-gray-300 hover:bg-gray-100'
  }`;

  // ---------- Vacío: sin sesión ----------
  if (estadoSesion === 'sin_sesion') {
    return (
      <div className="w-full max-w-xl mx-auto pt-6">
        {encabezado}
        <div className={`rounded-2xl border border-dashed p-8 text-center ${tema.tarjeta}`}>
          <h3 className="text-base font-semibold">No has iniciado sesión</h3>
          <p className={`mt-1 text-xs ${tema.textoSuave}`}>
            Iniciar sesión es opcional: la aplicación funciona igual sin usuario. Lo necesitarás para compartir gastos.
          </p>
          <div className="mt-5 flex flex-wrap items-center justify-center gap-3">
            <button type="button" onClick={() => setPantallaActual('inicio-sesion')} className={botonPrimario}>
              Iniciar sesión
            </button>
            <button type="button" onClick={() => setPantallaActual('registro')} className={botonSecundario}>
              Crear usuario
            </button>
          </div>
        </div>
      </div>
    );
  }

  // ---------- Sesión vencida ----------
  if (estadoSesion === 'vencida') {
    return (
      <div className="w-full max-w-xl mx-auto pt-6">
        {encabezado}
        <div role="status" className={`rounded-2xl border p-8 text-center ${tema.tarjeta}`}>
          <h3 className="text-base font-semibold">Tu sesión venció, inicia sesión de nuevo</h3>
          <p className={`mt-1 text-xs ${tema.textoSuave}`}>
            Tus cuentas, movimientos, tasas y patrimonio siguen disponibles.
          </p>
          <button type="button" onClick={() => setPantallaActual('inicio-sesion')} className={`mt-5 ${botonPrimario}`}>
            Iniciar sesión
          </button>
        </div>
      </div>
    );
  }

  // ---------- Cargando la sesión al abrir la app ----------
  if (estadoSesion === 'cargando') {
    return (
      <div className="w-full max-w-xl mx-auto pt-6" aria-busy="true">
        {encabezado}
        <div className={`h-40 animate-pulse rounded-2xl border ${tema.tarjeta}`} />
      </div>
    );
  }

  // ---------- Sesión vigente ----------
  const nombreVisible = control.sesion.nombreVisible ?? '';
  const inicial = nombreVisible.trim().charAt(0).toUpperCase() || '?';
  const usuario = estado.tipo === 'listo' ? estado.usuario : null;

  const filas: { etiqueta: string; valor: string }[] = [
    { etiqueta: 'Correo', valor: usuario?.correo ?? '—' },
    { etiqueta: 'Miembro desde', valor: formatearFecha(usuario?.fecha_alta, false) },
    { etiqueta: 'Esta sesión vence', valor: formatearFecha(control.sesion.venceEn, true) },
  ];

  return (
    <div className="w-full max-w-xl mx-auto pt-6">
      {encabezado}

      <div className={`rounded-2xl border p-6 shadow-sm ${tema.tarjeta}`} aria-busy={estado.tipo === 'cargando'}>
        <div className="flex items-center gap-4">
          <span
            className="flex h-14 w-14 shrink-0 items-center justify-center rounded-full bg-[#9333EA] text-xl font-bold text-white"
            aria-hidden="true"
          >
            {inicial}
          </span>
          <div className="min-w-0">
            <p className="truncate text-lg font-semibold">{nombreVisible}</p>
            <p className={`truncate text-xs ${tema.textoSuave}`}>
              {usuario ? `@${usuario.nombre_usuario}` : estado.tipo === 'cargando' ? 'Cargando tus datos…' : ''}
            </p>
          </div>
        </div>

        <dl className="mt-5 divide-y divide-gray-500/20 text-xs">
          {filas.map((f) => (
            <div key={f.etiqueta} className="grid grid-cols-1 gap-1 py-2.5 sm:grid-cols-[150px_minmax(0,1fr)]">
              <dt className={tema.textoSuave}>{f.etiqueta}</dt>
              <dd className="break-words font-medium">
                {estado.tipo === 'cargando' && f.etiqueta !== 'Esta sesión vence' ? (
                  <span className="inline-block h-3 w-32 animate-pulse rounded bg-gray-500/30" aria-hidden="true" />
                ) : (
                  f.valor
                )}
              </dd>
            </div>
          ))}
        </dl>

        {estado.tipo === 'error' && (
          <div
            role="alert"
            className="mt-4 flex flex-wrap items-center gap-3 rounded-xl border border-rose-800/60 bg-rose-950/40 p-3 text-xs text-rose-200"
          >
            <p className="min-w-0 flex-1">{estado.mensaje}</p>
            <button type="button" onClick={reintentar} className={botonSecundario}>
              Reintentar
            </button>
          </div>
        )}

        <div className="mt-5 flex flex-wrap items-center gap-3">
          <button type="button" onClick={cerrar} disabled={cerrando} className={`${botonSecundario} disabled:opacity-60`}>
            {cerrando ? 'Cerrando…' : 'Cerrar sesión'}
          </button>
          {errorCierre && (
            <p role="alert" className="text-[11px] font-medium text-rose-400">
              {errorCierre}
            </p>
          )}
        </div>
      </div>
    </div>
  );
};
