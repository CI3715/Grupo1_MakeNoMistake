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
  IconoCalendario,
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

  const itemsNav: { id: string; etiqueta: string; icono: React.ReactNode }[] = [
    {
      id: 'mis-cuentas',
      etiqueta: 'Cuentas',
      icono: <IconoBilletera className="h-4 w-4 shrink-0" />,
    },
    {
      id: 'detalle-cuenta',
      etiqueta: 'Detalle',
      icono: <IconoDetalleCuenta className="h-4 w-4 shrink-0" />,
    },
    {
      id: 'tasas',
      etiqueta: 'Tasas',
      icono: <IconoTasasCambio className="h-4 w-4 shrink-0" />,
    },
    {
      id: 'patrimonio',
      etiqueta: 'Patrimonio',
      icono: <IconoPatrimonio className="h-4 w-4 shrink-0" />,
    },
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
    {
      id: 'copia-seguridad',
      etiqueta: 'Copia',
      icono: <IconoCopiaSeguridad className="h-4 w-4 shrink-0" />,
    },
    {
      id: 'sincronizacion-libro',
      etiqueta: 'Sincro',
      icono: <IconoSincronizacion className="h-4 w-4 shrink-0" />,
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

  // ── OPCIÓN 2: Layout con Barra Lateral Flotante Expandible al Pasar el Mouse ──
  if (diseno === 'barra-lateral') {
    return (
      <div className="flex h-screen overflow-hidden bg-fondo text-texto transition-colors duration-200">
        {/* Barra Lateral Flotante que se expande al hover */}
        <Sidebar
          pantallaActual={pantallaActual}
          setPantallaActual={setPantallaActual}
          control={control}
          modoOscuro={modoOscuro}
        />

        {/* Área de trabajo de escritorio amplia con scroll suave */}
        <div className="flex flex-1 flex-col overflow-y-auto">
          {/* Barra de herramientas superior flotante redondeada */}
          <header className="sticky top-4 z-30 mx-4 sm:mx-8 flex h-14 shrink-0 items-center justify-between rounded-full border border-borde bg-superficie/95 px-6 backdrop-blur-md shadow-xs">
            <div className="min-w-0 pr-4">
              <h1 className="truncate text-base sm:text-lg font-extrabold tracking-tight text-texto">
                {infoEncabezado.titulo}
              </h1>
            </div>
            <div className="flex items-center gap-3 shrink-0">
              <span className="hidden md:inline-flex items-center gap-2 rounded-full border border-borde bg-superficie-2 px-3.5 py-1 text-xs font-semibold text-suave">
                <IconoCalendario className="h-3.5 w-3.5" />
                <span>
                  {new Date().toLocaleDateString('es-ES', {
                    day: 'numeric',
                    month: 'short',
                    year: 'numeric',
                  })}
                </span>
              </span>
              <ConmutadorDiseno disenoActual={diseno} variante="superficie" />
            </div>
          </header>

          <main className="w-full max-w-7xl mx-auto p-4 sm:p-8 space-y-6">
            {contenidoPrincipal}
          </main>
        </div>

        <BienvenidaPrimerAcceso modoOscuro={modoOscuro} setPantallaActual={setPantallaActual} />
      </div>
    );
  }

  // ── OPCIÓN 1: Layout con Bloque Curvado Superior y Degradado Distintivo ──
  return (
    <div className="min-h-screen bg-fondo pb-16 text-texto transition-colors duration-200">
      {/* Cabecera curvada distintiva con degradado */}
      <header className="relative rounded-b-[2.5rem] bg-linear-to-br from-grad-a to-grad-b pb-24 text-white shadow-lg transition-all duration-300">
        <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-4 pt-5 sm:px-8">
          {/* Fila superior: Logo y Controles */}
          <div className="flex items-center justify-between gap-4">
            <button
              type="button"
              onClick={() => setPantallaActual('mis-cuentas')}
              className="flex cursor-pointer items-center gap-3 text-left focus:outline-none"
            >
              <span className="grid size-11 place-items-center rounded-2xl bg-white/20 text-white shadow-xs backdrop-blur-md">
                <IconoLogo width={22} height={22} />
              </span>
              <div>
                <span className="block text-base font-extrabold tracking-tight text-white sm:text-lg">
                  Cuentas Claras
                </span>
                <span className="block text-[10px] font-bold tracking-widest text-white/80 uppercase">
                  LOCAL-FIRST
                </span>
              </div>
            </button>

            <div className="flex items-center gap-3">
              <ConmutadorDiseno disenoActual={diseno} variante="traslucido" />
              <SelectorApariencia variante="traslucido" />

              {sesionVigente ? (
                <button
                  type="button"
                  onClick={() => setPantallaActual('perfil')}
                  title={`Perfil: ${nombreVisible}`}
                  className="flex size-10 cursor-pointer items-center justify-center rounded-full bg-white/20 text-xs font-bold text-white shadow-xs ring-1 ring-white/30 backdrop-blur-md transition hover:bg-white/30"
                >
                  {inicialUsuario}
                </button>
              ) : (
                <button
                  type="button"
                  onClick={() => setPantallaActual('inicio-sesion')}
                  title="Iniciar sesión"
                  className="grid size-10 cursor-pointer place-items-center rounded-full bg-white/20 text-white shadow-xs ring-1 ring-white/30 backdrop-blur-md transition hover:bg-white/30"
                >
                  <IconoUsuario className="h-4 w-4" />
                </button>
              )}
            </div>
          </div>

          {/* Fila intermedia: Título de la vista y fecha */}
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 pt-1">
            <div>
              <h1 className="text-2xl font-extrabold tracking-tight text-white sm:text-3xl">
                {infoEncabezado.titulo}
              </h1>
              <p className="mt-0.5 text-xs text-white/80 sm:text-sm">
                {infoEncabezado.subtitulo}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <span className="inline-flex items-center gap-2 rounded-full bg-white/15 px-4 py-1.5 text-xs font-semibold text-white ring-1 ring-white/20 backdrop-blur-md shadow-xs">
                <IconoCalendario className="h-3.5 w-3.5" />
                <span>
                  {new Date().toLocaleDateString('es-ES', {
                    day: 'numeric',
                    month: 'short',
                    year: 'numeric',
                  })}
                </span>
              </span>
            </div>
          </div>

          {/* Fila de navegación en píldoras (caben todas sin scroll lateral) */}
          <div className="flex justify-center pt-1">
            <nav
              aria-label="Vistas principales"
              className="flex items-center justify-center flex-wrap gap-1 rounded-full bg-black/25 p-1.5 ring-1 ring-white/20 backdrop-blur-md max-w-full"
            >
              {itemsNav.map((item) => {
                const activo = pantallaActual === item.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setPantallaActual(item.id)}
                    className={`inline-flex cursor-pointer items-center gap-2 rounded-full px-4 py-2 text-xs font-semibold transition ${
                      activo
                        ? 'bg-white text-slate-950 shadow-md font-bold'
                        : 'text-white/80 hover:bg-white/15 hover:text-white'
                    }`}
                  >
                    {item.icono}
                    <span>{item.etiqueta}</span>
                  </button>
                );
              })}
            </nav>
          </div>
        </div>
      </header>

      {/* Contenido principal amplio superpuesto a la curva */}
      <main className="relative -mt-12 mx-auto w-full max-w-7xl px-4 sm:px-8">
        {contenidoPrincipal}
      </main>

      <BienvenidaPrimerAcceso modoOscuro={modoOscuro} setPantallaActual={setPantallaActual} />
    </div>
  );
}
