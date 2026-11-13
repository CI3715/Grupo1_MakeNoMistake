'use client';

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { CampoTexto } from '../sesion/CampoTexto';
import type { ContextoSocial, ErroresCampos, TipoFallo } from '../../types/social';
import { erroresDeAccion } from '../../utils/erroresSocial';
import { fechaHoyLocal, formatearMontoTexto, normalizarDecimal } from '../../utils/formato';
import { ETIQUETA_REGLA, REGLAS, signoBalance } from '../../utils/formatoSocial';
import {
  registrarGastoGrupoCommand,
  verGrupoCommand,
  type GrupoResumenDto,
  type MiembroDto,
  type ParticipantePayload,
  type ReglaDivision,
} from '../../utils/tauriCommands';
import { temaSocial } from '../../utils/temaSocial';
import { useCargaRemota, useMontado } from '../../utils/useCargaRemota';
import { useVistaPreviaDivision } from '../../utils/useVistaPreviaDivision';
import { AvisoFalloSocial } from './AvisoFalloSocial';
import { BotonAccion } from './BotonAccion';
import { CampoSelect } from './CampoSelect';
import { EncabezadoPantalla } from './EncabezadoPantalla';
import { EnlaceVolver } from './EnlaceVolver';
import { EstadoCargando } from './EstadoCargando';
import { MensajeGeneral } from './MensajeGeneral';
import { TablaParticipantes } from './TablaParticipantes';
import { Tarjeta } from './Tarjeta';

type CampoGasto = 'concepto' | 'monto' | 'fecha' | 'pagador' | 'regla' | 'participantes' | 'porcentajes' | 'montos';
const CAMPOS: readonly CampoGasto[] = [
  'concepto',
  'monto',
  'fecha',
  'pagador',
  'regla',
  'participantes',
  'porcentajes',
  'montos',
];
/** Orden en que se lleva el foco al primer campo con error. */
const FOCO: readonly { campos: readonly CampoGasto[]; id: string }[] = [
  { campos: ['concepto'], id: 'gasto-concepto' },
  { campos: ['monto'], id: 'gasto-monto' },
  { campos: ['fecha'], id: 'gasto-fecha' },
  { campos: ['pagador'], id: 'gasto-pagador' },
  { campos: ['regla', 'participantes', 'porcentajes', 'montos'], id: 'gasto-regla' },
];

interface Props {
  ctx: ContextoSocial;
  grupoId: string;
  onCancelar: () => void;
  onRegistrado: () => void;
}

/** Carga el grupo (moneda y miembros) y muestra el formulario cuando está listo. */
export function FormularioGasto({ ctx, grupoId, onCancelar, onRegistrado }: Props) {
  const cargar = useCallback(() => verGrupoCommand(grupoId), [grupoId]);
  const { carga, recargar } = useCargaRemota(cargar);

  return (
    <div className="@container">
      <EnlaceVolver modoOscuro={ctx.modoOscuro} texto="Volver al grupo" onClick={onCancelar} />
      {carga.estado === 'cargando' && <EstadoCargando modoOscuro={ctx.modoOscuro} que="el grupo" />}
      {carga.estado === 'fallo' && (
        <AvisoFalloSocial ctx={ctx} fallo={carga.fallo} onReintentar={() => recargar()} />
      )}
      {carga.estado === 'listo' && (
        <CuerpoFormulario
          ctx={ctx}
          grupo={carga.datos.grupo}
          miembros={carga.datos.miembros}
          onCancelar={onCancelar}
          onRegistrado={onRegistrado}
        />
      )}
    </div>
  );
}

interface CuerpoProps {
  ctx: ContextoSocial;
  grupo: GrupoResumenDto;
  miembros: MiembroDto[];
  onCancelar: () => void;
  onRegistrado: () => void;
}

function CuerpoFormulario({ ctx, grupo, miembros, onCancelar, onRegistrado }: CuerpoProps) {
  const [concepto, setConcepto] = useState('');
  const [monto, setMonto] = useState('');
  const [fecha, setFecha] = useState(fechaHoyLocal);
  const [pagador, setPagador] = useState('');
  const [regla, setRegla] = useState<ReglaDivision>('partes_iguales');
  // Por defecto entran todos; excluir a alguien es desmarcarlo.
  const [seleccion, setSeleccion] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(miembros.map((m) => [m.usuario, true]))
  );
  const [valores, setValores] = useState<Record<string, string>>({});
  const [enviando, setEnviando] = useState(false);
  const [errores, setErrores] = useState<ErroresCampos<CampoGasto>>({});
  const [tipoError, setTipoError] = useState<TipoFallo>('otro');
  // Con qué datos de la división se envió el gasto que el servidor rechazó (ver `delServidor`).
  const [firmaRechazada, setFirmaRechazada] = useState<string | null>(null);
  const montado = useMontado();
  const tema = temaSocial(ctx.modoOscuro);

  const participantes = useMemo<ParticipantePayload[]>(
    () =>
      miembros
        .filter((m) => seleccion[m.usuario])
        .map((m) => ({
          usuario: m.usuario,
          valor: regla === 'partes_iguales' ? '' : normalizarDecimal(valores[m.usuario] ?? ''),
        })),
    [miembros, seleccion, valores, regla]
  );

  const previa = useVistaPreviaDivision(monto, regla, participantes);
  const firmaDivision = JSON.stringify([monto, regla, participantes]);

  // Tras un error del envío, el foco va al primer campo que falló.
  useEffect(() => {
    const primero = FOCO.find((f) => f.campos.some((c) => errores[c]));
    if (primero) document.getElementById(primero.id)?.focus();
  }, [errores]);

  const enviar = async (e: React.FormEvent) => {
    e.preventDefault();
    if (enviando) return;
    setEnviando(true);
    setErrores({});
    try {
      await registrarGastoGrupoCommand({
        grupo_id: grupo.id,
        concepto,
        monto: normalizarDecimal(monto),
        fecha,
        pagador,
        regla,
        participantes,
      });
      if (montado.current) onRegistrado();
    } catch (err) {
      if (montado.current) {
        const r = erroresDeAccion(err, CAMPOS);
        setErrores(r.errores);
        setTipoError(r.tipo);
        setFirmaRechazada(firmaDivision);
      }
    } finally {
      if (montado.current) setEnviando(false);
    }
  };

  // Lo que dice la vista previa (en vivo) y, si el servidor lo rechazó, lo que dice el servidor.
  const diferenciaPrevia = previa.estado === 'lista' && !previa.datos.cuadra ? previa.datos.diferencia : null;
  const partes = previa.estado === 'lista' && previa.datos.cuadra ? previa.datos.partes : [];
  const erroresPrevia = previa.estado === 'error' ? previa.errores : null;
  // Los errores de la división que dio el servidor valen solo para los datos que se enviaron:
  // si la persona cambia el monto, la regla o un valor, dejan de mostrarse (la vista previa en
  // vivo ya dice cómo queda). Así no queda a la vista una diferencia que ya no es cierta.
  const vigenteServidor = firmaRechazada === firmaDivision;
  const delServidor = (campo: 'regla' | 'participantes' | 'porcentajes' | 'montos') =>
    vigenteServidor ? errores[campo] : undefined;
  const errorRegla = delServidor('regla') ?? erroresPrevia?.regla;
  // Un mismo texto no se repite: si la vista previa ya muestra esa diferencia, basta con ella.
  const erroresDivisionServidor = [
    delServidor('participantes'),
    delServidor('porcentajes'),
    delServidor('montos'),
  ].filter((m): m is string => !!m && m !== diferenciaPrevia);
  const avisoDatos =
    erroresPrevia?.participantes ?? erroresPrevia?.porcentajes ?? erroresPrevia?.montos ?? null;
  const errorMonto = errores.monto ?? (monto.trim() !== '' ? erroresPrevia?.monto : undefined);

  return (
    <>
      <EncabezadoPantalla
        modoOscuro={ctx.modoOscuro}
        titulo="Nuevo gasto compartido"
        subtitulo={`${grupo.nombre} · Nuevo gasto · Aún no cambia los balances.`}
      />

      <form onSubmit={enviar} noValidate>
        <div className="grid gap-4 @2xl:grid-cols-2">
          <Tarjeta modoOscuro={ctx.modoOscuro} titulo="¿Qué compartieron?">
            <div className="space-y-3">
              <CampoTexto
                id="gasto-concepto"
                etiqueta="Concepto"
                placeholder="Cena de bienvenida"
                autoComplete="off"
                valor={concepto}
                onCambio={setConcepto}
                error={errores.concepto}
                deshabilitado={enviando}
                modoOscuro={ctx.modoOscuro}
              />
              <div className="grid grid-cols-[minmax(0,1fr)_5rem] gap-3">
                <CampoTexto
                  id="gasto-monto"
                  etiqueta="Monto positivo"
                  placeholder="100,00"
                  inputMode="decimal"
                  autoComplete="off"
                  valor={monto}
                  onCambio={setMonto}
                  error={errorMonto}
                  deshabilitado={enviando}
                  modoOscuro={ctx.modoOscuro}
                />
                <CampoTexto
                  id="gasto-moneda"
                  etiqueta="Moneda"
                  valor={grupo.moneda}
                  onCambio={() => {}}
                  deshabilitado
                  modoOscuro={ctx.modoOscuro}
                />
              </div>
              <p className={`-mt-1 text-[11px] ${tema.textoSuave}`}>
                El gasto se registra en la moneda del grupo ({grupo.moneda}).
              </p>
              <CampoTexto
                id="gasto-fecha"
                etiqueta="Fecha"
                tipo="date"
                valor={fecha}
                onCambio={setFecha}
                error={errores.fecha}
                deshabilitado={enviando}
                modoOscuro={ctx.modoOscuro}
              />
              <CampoSelect
                id="gasto-pagador"
                etiqueta="¿Quién pagó?"
                valor={pagador}
                placeholder="Elige a la persona que pagó"
                opciones={miembros.map((m) => ({
                  valor: m.usuario,
                  etiqueta: `${m.nombre_visible} (@${m.usuario})`,
                }))}
                onCambio={setPagador}
                error={errores.pagador}
                deshabilitado={enviando}
                modoOscuro={ctx.modoOscuro}
              />
              <p className={`text-[11px] ${tema.textoSuave}`}>
                Los participantes pueden ser distintos de quien pagó. Excluye a quien no participó
                desmarcándolo a la derecha.
              </p>
            </div>
          </Tarjeta>

          <Tarjeta
            modoOscuro={ctx.modoOscuro}
            titulo="Regla de división"
            descripcion="Así se reparte el gasto entre los participantes."
          >
            <div className="space-y-3">
              <CampoSelect
                id="gasto-regla"
                etiqueta="Método"
                valor={regla}
                opciones={REGLAS.map((r) => ({ valor: r, etiqueta: ETIQUETA_REGLA[r] }))}
                onCambio={(v) => setRegla(v as ReglaDivision)}
                error={errorRegla}
                deshabilitado={enviando}
                modoOscuro={ctx.modoOscuro}
              />

              <TablaParticipantes
                modoOscuro={ctx.modoOscuro}
                miembros={miembros}
                moneda={grupo.moneda}
                regla={regla}
                seleccion={seleccion}
                valores={valores}
                partes={partes}
                deshabilitado={enviando}
                onSeleccion={(u, incluido) => setSeleccion((s) => ({ ...s, [u]: incluido }))}
                onValor={(u, v) => setValores((x) => ({ ...x, [u]: v }))}
              />

              {/* Vista previa en vivo: se anuncia a lectores de pantalla sin interrumpir. */}
              <div aria-live="polite" className="space-y-2 text-xs">
                {previa.estado === 'inactiva' && (
                  <p className={tema.textoSuave}>
                    Escribe el monto y elige participantes para ver cuánto le toca a cada uno.
                  </p>
                )}

                {previa.estado === 'lista' && previa.datos.cuadra && (
                  <div className={`rounded-xl border p-3 ${tema.avisoMorado}`}>
                    {signoBalance(previa.datos.residuo) === 'cero' ? (
                      <p className="font-semibold">El reparto es exacto: no hay redondeo.</p>
                    ) : (
                      <>
                        <p className="font-semibold">
                          Residuo de redondeo: {formatearMontoTexto(previa.datos.residuo, grupo.moneda)}
                        </p>
                        <p className="mt-0.5 opacity-90">
                          Se reparte de a un céntimo por orden alfabético de usuario. Ningún céntimo
                          se pierde.
                        </p>
                      </>
                    )}
                  </div>
                )}

                {diferenciaPrevia && (
                  <div className={`rounded-xl border p-3 ${tema.avisoError}`}>
                    <p className="font-semibold">{diferenciaPrevia}</p>
                    <p className="mt-0.5 opacity-90">Ajusta los valores para que la suma cuadre.</p>
                  </div>
                )}

                {avisoDatos && !diferenciaPrevia && (
                  <p className={`rounded-xl border p-3 ${tema.avisoAmbar}`}>{avisoDatos}</p>
                )}
              </div>

              {erroresDivisionServidor.map((mensaje) => (
                <p key={mensaje} role="alert" className="text-[11px] font-medium text-rose-400">
                  {mensaje}
                </p>
              ))}
            </div>
          </Tarjeta>
        </div>

        {errores.general && (
          <div className="mt-4">
            <MensajeGeneral ctx={ctx} mensaje={errores.general} tipo={tipoError} />
          </div>
        )}

        <div className="mt-4 flex flex-wrap justify-end gap-2">
          <BotonAccion
            modoOscuro={ctx.modoOscuro}
            variante="secundario"
            deshabilitado={enviando}
            onClick={onCancelar}
          >
            Cancelar
          </BotonAccion>
          <BotonAccion modoOscuro={ctx.modoOscuro} tipo="submit" cargando={enviando}>
            {enviando ? 'Registrando…' : 'Registrar gasto compartido'}
          </BotonAccion>
        </div>
      </form>
    </>
  );
}
