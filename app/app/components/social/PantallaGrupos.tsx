'use client';

import React, { useCallback, useState } from 'react';
import type { ContextoSocial, VistaGrupos } from '../../types/social';
import { plural } from '../../utils/formatoSocial';
import { listarGruposCommand } from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useCargaRemota } from '../../utils/useCargaRemota';
import { AvisoFalloSocial } from './AvisoFalloSocial';
import { DetalleGrupo } from './DetalleGrupo';
import { EncabezadoPantalla } from './EncabezadoPantalla';
import { EstadoCargando } from './EstadoCargando';
import { FormularioCrearGrupo } from './FormularioCrearGrupo';
import { FormularioGasto } from './FormularioGasto';
import { InvitacionesGrupo } from './InvitacionesGrupo';
import { ListaGrupos } from './ListaGrupos';

/**
 * «Grupos»: lista (con invitaciones y formulario de creación), detalle de un grupo y
 * formulario de nuevo gasto. La subpantalla es estado local (RES-06); no hay rutas.
 */
export function PantallaGrupos({ ctx }: { ctx: ContextoSocial }) {
  const [vista, setVista] = useState<VistaGrupos>({ tipo: 'lista' });
  const [aviso, setAviso] = useState<string | null>(null);

  const irALista = useCallback(() => {
    setAviso(null);
    setVista({ tipo: 'lista' });
  }, []);

  if (vista.tipo === 'detalle') {
    return (
      <DetalleGrupo
        key={vista.grupoId}
        ctx={ctx}
        grupoId={vista.grupoId}
        aviso={aviso}
        onVolver={irALista}
        onNuevoGasto={() => {
          setAviso(null);
          setVista({ tipo: 'nuevo-gasto', grupoId: vista.grupoId });
        }}
      />
    );
  }

  if (vista.tipo === 'nuevo-gasto') {
    return (
      <FormularioGasto
        key={vista.grupoId}
        ctx={ctx}
        grupoId={vista.grupoId}
        onCancelar={() => setVista({ tipo: 'detalle', grupoId: vista.grupoId })}
        onRegistrado={() => {
          setAviso('Gasto registrado. La división y los balances ya están actualizados.');
          setVista({ tipo: 'detalle', grupoId: vista.grupoId });
        }}
      />
    );
  }

  return <ListaDeGrupos ctx={ctx} onAbrir={(grupoId) => setVista({ tipo: 'detalle', grupoId })} />;
}

function ListaDeGrupos({ ctx, onAbrir }: { ctx: ContextoSocial; onAbrir: (grupoId: string) => void }) {
  const { carga, recargar } = useCargaRemota(listarGruposCommand);
  const tema = temaSocial(ctx.modoOscuro);
  const actualizar = () => recargar(false);

  return (
    <div className="@container">
      <EncabezadoPantalla
        modoOscuro={ctx.modoOscuro}
        titulo="Compartir, sin enredos"
        subtitulo="Grupos · Tus actividades y gastos en común."
      />

      {carga.estado === 'cargando' && <EstadoCargando modoOscuro={ctx.modoOscuro} que="tus grupos" />}

      {carga.estado === 'fallo' && (
        <AvisoFalloSocial ctx={ctx} fallo={carga.fallo} onReintentar={() => recargar()} />
      )}

      {carga.estado === 'listo' && (
        <>
          <dl className="mb-4 grid grid-cols-2 gap-3">
            <ResumenCifra
              ctx={ctx}
              titulo="Grupos activos"
              valor={plural(carga.datos.grupos.length, 'grupo', 'grupos')}
              pie="Cada grupo con su propia moneda"
            />
            <ResumenCifra
              ctx={ctx}
              titulo="Por revisar"
              valor={plural(carga.datos.invitaciones.length, 'invitación', 'invitaciones')}
              pie={
                carga.datos.invitaciones.length === 0
                  ? 'Ninguna pendiente'
                  : 'No entras hasta aceptar'
              }
            />
          </dl>

          <div className="grid gap-4 @2xl:grid-cols-[minmax(0,1fr)_minmax(0,20rem)]">
            <div className="min-w-0 space-y-4">
              <ListaGrupos ctx={ctx} grupos={carga.datos.grupos} onAbrir={onAbrir} />
              <InvitacionesGrupo
                ctx={ctx}
                invitaciones={carga.datos.invitaciones}
                onCambio={actualizar}
              />
            </div>
            <div className="min-w-0">
              <FormularioCrearGrupo ctx={ctx} onCreado={(g) => onAbrir(g.id)} />
            </div>
          </div>
        </>
      )}
      <p className={`mt-6 text-[10px] uppercase tracking-wide ${tema.textoSuave}`}>
        Cuentas Claras · Datos locales · Sin bancos conectados
      </p>
    </div>
  );
}

function ResumenCifra({
  ctx,
  titulo,
  valor,
  pie,
}: {
  ctx: ContextoSocial;
  titulo: string;
  valor: string;
  pie: string;
}) {
  const tema = temaSocial(ctx.modoOscuro);
  return (
    <div className={`min-w-0 rounded-2xl border p-4 shadow-sm ${tema.tarjeta}`}>
      <dt className={`text-[11px] ${tema.textoSuave}`}>{titulo}</dt>
      <dd className="mt-1 text-xl font-bold tracking-tight">{valor}</dd>
      <dd className={`mt-1 text-[11px] ${tema.textoSuave}`}>{pie}</dd>
    </div>
  );
}
