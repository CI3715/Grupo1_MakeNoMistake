'use client';

import React, { Fragment, useState, useSyncExternalStore } from 'react';
import { PantallaCuentas } from './components/PantallaCuentas';
import { PantallaTasas } from './components/PantallaTasas';
import { PantallaPatrimonio } from './components/PantallaPatrimonio';
import { PantallaSesion } from './components/sesion/PantallaSesion';
import { PantallaSocial } from './components/social/PantallaSocial';
import { PantallaCopiaSeguridad } from './components/copia/PantallaCopiaSeguridad';
import { PantallaSincronizacionLibro } from './components/sincronizacionLibro/PantallaSincronizacionLibro';
import { BienvenidaPrimerAcceso } from './components/sesion/BienvenidaPrimerAcceso';
import { AvisoSesionVencida } from './components/sesion/AvisoSesionVencida';
import { SelectorApariencia } from './components/SelectorApariencia';
import { ConmutadorDiseno } from './components/ConmutadorDiseno';
import { Sidebar } from './components/Sidebar';
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
} from './components/Iconos';
import {
  type DisenoEscritorio,
  leerSnapshotDiseno,
  leerSnapshotOscuro,
  suscribirTema,
} from './utils/apariencia';
import { useSesion } from './utils/useSesion';
import { useSincronizacion } from './utils/useSincronizacion';
import { useSincronizacionLibro } from './utils/useSincronizacionLibro';

const ENCABEZADOS: Record<string, { titulo: string; subtitulo: string }> = {
  'mis-cuentas': {
    titulo: 'Mis cuentas',
    subtitulo: 'Todas tus cuentas y saldos guardados localmente en este dispositivo.',
  },
  'detalle-cuenta': {
    titulo: 'Detalle de cuenta',
    subtitulo: 'Libro de movimientos, composición de saldo y categorías.',
  },
  tasas: {
    titulo: 'Tasas de cambio',
    subtitulo: 'Equivalencias entre monedas con su fecha y fuente correspondiente.',
  },
  patrimonio: {
    titulo: 'Resumen de patrimonio',
    subtitulo: 'Estado de activos, monedas personalizadas y total consolidado.',
  },
  amigos: {
    titulo: 'Amigos',
    subtitulo: 'Tu lista de amigos e invitaciones para compartir gastos.',
  },
  grupos: {
    titulo: 'Grupos',
    subtitulo: 'Gastos compartidos, divisiones, saldos y sincronización con el servidor.',
  },
  'copia-seguridad': {
    titulo: 'Copia de seguridad',
    subtitulo: 'Exporta o importa tu libro completo con cifrado Argon2id + AES-256-GCM.',
  },
  'sincronizacion-libro': {
    titulo: 'Sincronización entre dispositivos',
    subtitulo:
      'Mantén tu libro al día entre tus dispositivos mediante carpeta compartida o repositorio Git.',
  },
  'inicio-sesion': {
    titulo: 'Iniciar sesión',
    subtitulo: 'Conéctate cuando quieras usar amigos y grupos. Tu libro sigue siendo local.',
  },
  registro: {
    titulo: 'Crear usuario',
    subtitulo: 'Crea una identidad para compartir gastos sin exponer tus cuentas personales.',
  },
  perfil: {
    titulo: 'Mi perfil',
    subtitulo:
      'Tu identidad en el servidor. Tus cuentas y movimientos nunca salen de tu dispositivo.',
  },
};

export default function Home() {
  const [pantallaActual, setPantallaActual] = useState<string>('mis-cuentas');
  const diseno: DisenoEscritorio = useSyncExternalStore(
    suscribirTema,
    leerSnapshotDiseno,
    () => 'barra-superior'
  );
  const modoOscuro = useSyncExternalStore(suscribirTema, leerSnapshotOscuro, () => true);
  const control = useSesion();
  const sincronizacion = useSincronizacion({
    sesionEstado: control.sesion.estado,
    claveLibro: control.claveLibro,
  });
  // Sincronización del libro entre dispositivos propios (Entrega 8).
  const refrescarLibro = sincronizacion.refrescar;
  const sincronizacionLibro = useSincronizacionLibro({
    claveLibro: control.claveLibro,
    alCambiarLibro: () => void refrescarLibro(),
  });

  // Mapeamos el estado principal al tipo exacto esperado por PantallaCuentas
  const vistaSubpantalla: 'mis-cuentas' | 'detalle-cuenta' =
    pantallaActual === 'detalle-cuenta' ? 'detalle-cuenta' : 'mis-cuentas';

  const handleCambiarSubpantalla = (
    vista: 'mis-cuentas' | 'detalle-cuenta' | 'tasas' | 'patrimonio'
  ) => {
    setPantallaActual(vista);
  };

  const infoEncabezado = ENCABEZADOS[pantallaActual] ?? ENCABEZADOS['mis-cuentas'];
  const sesionVigente = control.sesion.estado === 'vigente';
  const nombreVisible = control.sesion.nombreVisible ?? 'Mi perfil';
  const inicialUsuario = nombreVisible.trim().charAt(0).toUpperCase() || 'U';
  const enSesionOPerfil =
    pantallaActual === 'inicio-sesion' ||
    pantallaActual === 'registro' ||
    pantallaActual === 'perfil';

  const itemsNav: { id: string; etiqueta: string; icono: React.ReactNode }[] = [
    {
      id: 'mis-cuentas',
      etiqueta: 'Mis cuentas',
      icono: <IconoBilletera className="h-3.5 w-3.5 shrink-0" />,
    },
    {
      id: 'detalle-cuenta',
      etiqueta: 'Detalle',
      icono: <IconoDetalleCuenta className="h-3.5 w-3.5 shrink-0" />,
    },
    {
      id: 'tasas',
      etiqueta: 'Tasas',
      icono: <IconoTasasCambio className="h-3.5 w-3.5 shrink-0" />,
    },
    {
      id: 'patrimonio',
      etiqueta: 'Patrimonio',
      icono: <IconoPatrimonio className="h-3.5 w-3.5 shrink-0" />,
    },
    {
      id: 'amigos',
      etiqueta: 'Amigos',
      icono: <IconoAmigos className="h-3.5 w-3.5 shrink-0" />,
    },
    {
      id: 'grupos',
      etiqueta: 'Grupos',
      icono: <IconoRecibo className="h-3.5 w-3.5 shrink-0" />,
    },
    {
      id: 'copia-seguridad',
      etiqueta: 'Copia de seguridad',
      icono: <IconoCopiaSeguridad className="h-3.5 w-3.5 shrink-0" />,
    },
    {
      id: 'sincronizacion-libro',
      etiqueta: 'Sincronización',
      icono: <IconoSincronizacion className="h-3.5 w-3.5 shrink-0" />,
    },
  ];

  const contenidoPrincipal = (
    <>
      {control.avisoVencidaVisible && (
        <AvisoSesionVencida
          modoOscuro={modoOscuro}
          mostrarBoton={pantallaActual !== 'inicio-sesion'}
          onIniciarSesion={() => setPantallaActual('inicio-sesion')}
          onCerrar={control.ocultarAviso}
        />
      )}

      {/* Al iniciar o cerrar sesión cambia la clave: las pantallas se montan de nuevo y piden
          sus datos al libro nuevo, sin dejar a la vista nada del usuario anterior. Las pantallas
          de sesión quedan fuera para no desmontar el formulario que está iniciando sesión. */}
      <Fragment key={control.claveLibro}>
        {(pantallaActual === 'mis-cuentas' || pantallaActual === 'detalle-cuenta') && (
          <PantallaCuentas
            modoOscuro={modoOscuro}
            vistaSubpantalla={vistaSubpantalla}
            setVistaSubpantalla={handleCambiarSubpantalla}
            versionDatos={sincronizacion.versionDatos}
            diseno={diseno}
          />
        )}
        {pantallaActual === 'tasas' && <PantallaTasas modoOscuro={modoOscuro} />}
        {pantallaActual === 'patrimonio' && (
          <PantallaPatrimonio
            modoOscuro={modoOscuro}
            versionDatos={sincronizacion.versionDatos}
          />
        )}
        {(pantallaActual === 'amigos' || pantallaActual === 'grupos') && (
          <div className="rounded-3xl border border-borde bg-superficie p-5 shadow-suave sm:p-6">
            <PantallaSocial
              modoOscuro={modoOscuro}
              vista={pantallaActual}
              setPantallaActual={setPantallaActual}
              sincronizacion={sincronizacion}
            />
          </div>
        )}
        {pantallaActual === 'copia-seguridad' && (
          <div className="rounded-3xl border border-borde bg-superficie p-5 shadow-suave sm:p-6">
            <PantallaCopiaSeguridad
              modoOscuro={modoOscuro}
              alCambiarLibro={() => void refrescarLibro()}
            />
          </div>
        )}
        {pantallaActual === 'sincronizacion-libro' && (
          <div className="rounded-3xl border border-borde bg-superficie p-5 shadow-suave sm:p-6">
            <PantallaSincronizacionLibro
              modoOscuro={modoOscuro}
              control={sincronizacionLibro}
            />
          </div>
        )}
      </Fragment>

      {(pantallaActual === 'inicio-sesion' ||
        pantallaActual === 'registro' ||
        pantallaActual === 'perfil') && (
        <div className="rounded-3xl border border-borde bg-superficie p-5 shadow-suave sm:p-6">
          <PantallaSesion
            modoOscuro={modoOscuro}
            vista={pantallaActual}
            setPantallaActual={setPantallaActual}
            control={control}
          />
        </div>
      )}
    </>
  );

  // ── OPCIÓN 2: Layout con Barra Lateral Fija de Escritorio ──
  if (diseno === 'barra-lateral') {
    return (
      <div className="flex h-screen overflow-hidden bg-fondo text-texto transition-colors duration-200">
        <Sidebar
          pantallaActual={pantallaActual}
          setPantallaActual={setPantallaActual}
          control={control}
          modoOscuro={modoOscuro}
        />

        <div className="flex flex-1 flex-col overflow-y-auto">
          {/* Barra superior de herramientas de escritorio */}
          <header className="sticky top-0 z-30 flex h-16 shrink-0 items-center justify-between border-b border-borde bg-superficie/85 px-6 backdrop-blur-md sm:px-8">
            <div className="min-w-0 pr-4">
              <h1 className="truncate text-base sm:text-lg font-bold tracking-tight text-texto">
                {infoEncabezado.titulo}
              </h1>
              <p className="hidden sm:block truncate text-xs text-suave">
                {infoEncabezado.subtitulo}
              </p>
            </div>
            <div className="flex items-center gap-3 shrink-0">
              <ConmutadorDiseno
                disenoActual={diseno}
                variante="superficie"
              />
            </div>
          </header>

          <main className="w-full max-w-7xl mx-auto p-6 sm:p-8">
            {contenidoPrincipal}
          </main>
        </div>

        <BienvenidaPrimerAcceso modoOscuro={modoOscuro} setPantallaActual={setPantallaActual} />
      </div>
    );
  }

  // ── OPCIONES 1 & 3: Layouts con Barra Superior Amplia o Dividida ──
  return (
    <div className="min-h-screen bg-fondo pb-16 text-texto transition-colors duration-200">
      {/* Cabecera superior curva con degradado dinámico ampliada para escritorio */}
      <div className="relative rounded-b-[2.5rem] bg-linear-to-br from-grad-a to-grad-b pb-28 text-white shadow-lg">
        {/* Esferas decorativas de luz y desenfoque ambiental */}
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 overflow-hidden rounded-b-[2.5rem]"
        >
          <div className="absolute -right-24 -top-32 size-96 rounded-full bg-white/10 blur-2xl" />
          <div className="absolute -bottom-44 left-1/3 size-96 rounded-full bg-white/5 blur-3xl" />
        </div>

        <div className="relative mx-auto w-full max-w-7xl px-4 pt-5 sm:px-8 sm:pt-6">
          {/* Fila 1: Marca + Selector de Disposición + Sesión + Selector de Apariencia */}
          <header className="flex flex-col gap-4">
            <div className="flex items-center justify-between gap-3">
              <button
                type="button"
                onClick={() => setPantallaActual('mis-cuentas')}
                className="flex cursor-pointer items-center gap-2.5 text-left focus:outline-none"
              >
                <span className="grid size-9 place-items-center rounded-2xl bg-white/15 shadow-sm ring-1 ring-white/25 backdrop-blur-xs sm:size-10">
                  <IconoLogo width={20} height={20} />
                </span>
                <div>
                  <span className="text-base font-bold tracking-tight sm:text-lg">
                    Cuentas Claras
                  </span>
                  <span className="ml-2 hidden rounded-md bg-white/15 px-2 py-0.5 text-[10px] font-semibold tracking-wider text-white/90 sm:inline">
                    LOCAL-FIRST
                  </span>
                </div>
              </button>

              <div className="flex items-center gap-2.5 flex-wrap justify-end">
                {/* Conmutador rápido de diseño de escritorio */}
                <ConmutadorDiseno
                  disenoActual={diseno}
                  variante="traslucido"
                />

                {sesionVigente ? (
                  <button
                    type="button"
                    onClick={() => setPantallaActual('perfil')}
                    className={`inline-flex h-9 cursor-pointer items-center gap-2 rounded-xl px-3 text-xs font-semibold transition sm:h-10 sm:px-3.5 ${
                      pantallaActual === 'perfil'
                        ? 'bg-white text-slate-950 shadow-sm'
                        : 'bg-white/15 text-white ring-1 ring-white/25 backdrop-blur hover:bg-white/25'
                    }`}
                  >
                    <span className="flex size-5 items-center justify-center rounded-full bg-acento text-[11px] font-bold text-acento-fg">
                      {inicialUsuario}
                    </span>
                    <span className="max-w-[120px] truncate">{nombreVisible}</span>
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => setPantallaActual('inicio-sesion')}
                    className={`inline-flex h-9 cursor-pointer items-center gap-1.5 rounded-xl px-3 text-xs font-semibold transition sm:h-10 sm:px-3.5 ${
                      enSesionOPerfil
                        ? 'bg-white text-slate-950 shadow-sm'
                        : 'bg-white/15 text-white ring-1 ring-white/25 backdrop-blur hover:bg-white/25'
                    }`}
                  >
                    <IconoUsuario className="h-3.5 w-3.5 shrink-0" />
                    <span>Iniciar sesión</span>
                  </button>
                )}

                <SelectorApariencia />
              </div>
            </div>

            {/* Fila 2: Píldora de navegación entre todas las vistas de la aplicación */}
            <nav
              aria-label="Vistas principales"
              className="flex flex-wrap items-center gap-1 rounded-2xl bg-black/25 p-1.5 ring-1 ring-white/20 backdrop-blur-md"
            >
              {itemsNav.map((item) => {
                const activo = pantallaActual === item.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setPantallaActual(item.id)}
                    className={`inline-flex cursor-pointer items-center gap-1.5 rounded-xl px-3 py-1.5 text-xs font-bold transition sm:text-sm ${
                      activo
                        ? 'bg-white text-slate-950 shadow-sm'
                        : 'text-white/85 hover:bg-white/10 hover:text-white'
                    }`}
                  >
                    {item.icono}
                    <span>{item.etiqueta}</span>
                  </button>
                );
              })}
            </nav>
          </header>

          {/* Título dinámico en la cabecera */}
          <div className="mt-6 sm:mt-7">
            <h1 className="text-2xl font-bold tracking-tight sm:text-4xl">
              {infoEncabezado.titulo}
            </h1>
            <p className="mt-1 text-xs text-white/80 sm:mt-1.5 sm:text-sm">
              {infoEncabezado.subtitulo}
            </p>
          </div>
        </div>
      </div>

      {/* Contenido principal superpuesto a la cabecera con margen negativo y ancho amplio */}
      <main className="relative mx-auto -mt-14 w-full max-w-7xl px-4 sm:-mt-16 sm:px-8">
        {contenidoPrincipal}
      </main>

      <BienvenidaPrimerAcceso modoOscuro={modoOscuro} setPantallaActual={setPantallaActual} />
    </div>
  );
}
