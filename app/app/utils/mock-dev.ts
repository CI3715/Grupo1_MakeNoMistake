/**
 * MOCK INTERACTIVO PARA DISEÑO Y PRUEBAS EN EL NAVEGADOR (npm run dev → http://localhost:3000)
 *
 * Simula todos los comandos de Tauri con `mockIPC`:
 *   - listar_monedas
 *   - listar_cuentas
 *   - crear_cuenta
 *   - listar_categorias
 *   - crear_categoria
 *   - detalle_cuenta
 *   - registrar_movimiento
 *   - previsualizar_movimiento
 *   - corregir_movimiento
 *
 * No hace nada si la app corre dentro de la ventana de Tauri (`window.__TAURI_INTERNALS__`).
 */

import { mockIPC } from '@tauri-apps/api/mocks';
import type {
  Cuenta,
  Categoria,
  Movimiento,
  Moneda,
  NuevaCuenta,
  Subtotal,
  TipoCuenta,
  DetalleCuentaData,
} from '../types/cuentas';

const w = typeof window !== 'undefined'
  ? (window as unknown as { __TAURI_INTERNALS__?: unknown; __MOCK_CC__?: boolean })
  : undefined;

// Solo activo si NO está corriendo dentro de Tauri
const ACTIVO = typeof window !== 'undefined' && (!w || !w.__TAURI_INTERNALS__ || w.__MOCK_CC__);

const MONEDAS: Moneda[] = [
  { codigo: 'USD', nombre: 'Dólar estadounidense', simbolo: '$', decimales: 2 },
  { codigo: 'VES', nombre: 'Bolívar', simbolo: 'Bs.', decimales: 2 },
  { codigo: 'EUR', nombre: 'Euro', simbolo: '€', decimales: 2 },
  { codigo: 'USDT', nombre: 'Tether (dólar digital)', simbolo: '₮', decimales: 2 },
];

const CUENTAS_INICIALES: Cuenta[] = [
  { id: 'cuenta-1', nombre: 'Banco Mercantil', tipo: 'banco', moneda: 'VES', saldo_actual: '4250.00' },
  { id: 'cuenta-2', nombre: 'Banesco Panamá', tipo: 'banco', moneda: 'USD', saldo_actual: '350.00' },
  { id: 'cuenta-3', nombre: 'Efectivo Cartera', tipo: 'efectivo', moneda: 'USD', saldo_actual: '45.00' },
  { id: 'cuenta-4', nombre: 'Billetera Binance', tipo: 'billetera_digital', moneda: 'USDT', saldo_actual: '120.00' },
];

const CATEGORIAS_INICIALES: Categoria[] = [
  { id: 'cat-1', nombre: 'Alimentación', tipo: 'gasto', de_fabrica: true },
  { id: 'cat-2', nombre: 'Transporte', tipo: 'gasto', de_fabrica: true },
  { id: 'cat-3', nombre: 'Servicios', tipo: 'gasto', de_fabrica: true },
  { id: 'cat-4', nombre: 'Salud', tipo: 'gasto', de_fabrica: true },
  { id: 'cat-5', nombre: 'Educación', tipo: 'gasto', de_fabrica: true },
  { id: 'cat-6', nombre: 'Entretenimiento', tipo: 'gasto', de_fabrica: true },
  { id: 'cat-7', nombre: 'Otros gastos', tipo: 'gasto', de_fabrica: true },
  { id: 'cat-8', nombre: 'Salario', tipo: 'ingreso', de_fabrica: true },
  { id: 'cat-9', nombre: 'Ventas', tipo: 'ingreso', de_fabrica: true },
  { id: 'cat-10', nombre: 'Regalos', tipo: 'ingreso', de_fabrica: true },
  { id: 'cat-11', nombre: 'Otros ingresos', tipo: 'ingreso', de_fabrica: true },
];

const MOVIMIENTOS_INICIALES: Record<string, Movimiento[]> = {
  'cuenta-1': [
    {
      id: 'mov-1',
      cuenta_id: 'cuenta-1',
      tipo: 'ingreso',
      monto: '5000.00',
      monto_con_signo: '+ 5000.00',
      fecha: '2026-10-20',
      categoria_id: 'cat-5',
      categoria_nombre: 'Salario / Nómina',
      nota: 'Pago quincena octubre',
      estado: 'normal',
      ajustes: [],
    },
    {
      id: 'mov-2',
      cuenta_id: 'cuenta-1',
      tipo: 'gasto',
      monto: '750.00',
      monto_con_signo: '− 750.00',
      fecha: '2026-10-22',
      categoria_id: 'cat-1',
      categoria_nombre: 'Supermercado',
      nota: 'Compras semanales',
      estado: 'normal',
      ajustes: [],
    },
  ],
  'cuenta-2': [
    {
      id: 'mov-3',
      cuenta_id: 'cuenta-2',
      tipo: 'ingreso',
      monto: '400.00',
      monto_con_signo: '+ 400.00',
      fecha: '2026-10-15',
      categoria_id: 'cat-7',
      categoria_nombre: 'Venta de Servicios',
      nota: 'Consultoría software',
      estado: 'normal',
      ajustes: [],
    },
    {
      id: 'mov-4',
      cuenta_id: 'cuenta-2',
      tipo: 'gasto',
      monto: '50.00',
      monto_con_signo: '− 50.00',
      fecha: '2026-10-18',
      categoria_id: 'cat-4',
      categoria_nombre: 'Servicios Básicos',
      nota: 'Internet fibra',
      estado: 'normal',
      ajustes: [],
    },
  ],
};

function aCentavos(texto: string): number {
  const [entero, frac = ''] = texto.split('.');
  return Number(entero) * 100 + Number(frac.padEnd(2, '0').slice(0, 2));
}

function calcularSubtotales(cuentas: Cuenta[]): Subtotal[] {
  const orden: string[] = [];
  const totales: Record<string, number> = {};
  for (const c of cuentas) {
    if (!(c.moneda in totales)) {
      orden.push(c.moneda);
      totales[c.moneda] = 0;
    }
    totales[c.moneda] += aCentavos(c.saldo_actual);
  }
  return orden.map((moneda) => ({ moneda, total: (totales[moneda] / 100).toFixed(2) }));
}

if (ACTIVO && w) {
  w.__MOCK_CC__ = true;
  const cuentas = [...CUENTAS_INICIALES];
  const categorias = [...CATEGORIAS_INICIALES];
  const movimientosMap: Record<string, Movimiento[]> = { ...MOVIMIENTOS_INICIALES };
  let contador = 100;

  mockIPC(async (cmd, args) => {
    // 1. Listar monedas
    if (cmd === 'listar_monedas') {
      return MONEDAS;
    }

    // 2. Listar cuentas
    if (cmd === 'listar_cuentas') {
      return {
        cuentas: [...cuentas],
        subtotales: calcularSubtotales(cuentas),
      };
    }

    // 3. Crear cuenta
    if (cmd === 'crear_cuenta') {
      const datos = (args as { datos: NuevaCuenta }).datos;
      const [entero, frac = ''] = datos.saldo_inicial.split('.');
      const saldoFinal = `${entero}.${frac.padEnd(2, '0')}`;
      const nueva: Cuenta = {
        id: `cuenta-${++contador}`,
        nombre: datos.nombre.trim(),
        tipo: datos.tipo as TipoCuenta,
        moneda: datos.moneda,
        saldo_actual: saldoFinal,
      };
      cuentas.push(nueva);
      movimientosMap[nueva.id] = [];
      return nueva;
    }

    // 4. Listar categorías
    if (cmd === 'listar_categorias') {
      return categorias;
    }

    // 5. Crear categoría
    if (cmd === 'crear_categoria') {
      const datos = (args as { datos: { nombre: string; tipo: 'gasto' | 'ingreso' } }).datos;
      const nuevaCat: Categoria = {
        id: `cat-${++contador}`,
        nombre: datos.nombre,
        tipo: datos.tipo,
        de_fabrica: false,
      };
      categorias.push(nuevaCat);
      return nuevaCat;
    }

    // 6. Detalle cuenta
    if (cmd === 'detalle_cuenta') {
      const cuentaId = (args as { cuenta_id: string }).cuenta_id;
      const c = cuentas.find((item) => item.id === cuentaId);
      if (!c) throw new Error('Cuenta no encontrada');

      const movs = movimientosMap[cuentaId] || [];

      // Calcular composición
      let totalIngresos = 0;
      let totalGastos = 0;
      let totalAjustes = 0;

      for (const m of movs) {
        const cent = aCentavos(m.monto);
        if (m.estado === 'ajuste') {
          totalAjustes += cent;
        } else if (m.tipo.toLowerCase() === 'ingreso') {
          totalIngresos += cent;
        } else {
          totalGastos += cent;
        }
      }

      const composicion = {
        saldo_inicial: c.saldo_actual,
        total_ingresos: (totalIngresos / 100).toFixed(2),
        total_gastos: (totalGastos / 100).toFixed(2),
        total_ajustes: (totalAjustes / 100).toFixed(2),
      };

      const resultado: DetalleCuentaData = {
        cuenta: c,
        composicion,
        movimientos: [...movs],
      };
      return resultado;
    }

    // 7. Previsualizar movimiento
    if (cmd === 'previsualizar_movimiento') {
      const datos = (args as { datos: { cuenta_id: string; tipo: string; monto: string } }).datos;
      const c = cuentas.find((item) => item.id === datos.cuenta_id);
      const saldoCent = c ? aCentavos(c.saldo_actual) : 0;
      const montoCent = aCentavos(datos.monto);

      const resultante = datos.tipo.toLowerCase() === 'gasto' ? saldoCent - montoCent : saldoCent + montoCent;
      const dejaNegativo = resultante < 0;

      return {
        saldo_resultante: (resultante / 100).toFixed(2),
        deja_saldo_negativo: dejaNegativo,
      };
    }

    // 8. Registrar movimiento
    if (cmd === 'registrar_movimiento') {
      const datos = (args as {
        datos: {
          cuenta_id: string;
          tipo: string;
          monto: string;
          fecha: string;
          categoria_id: string;
          nota: string;
        };
      }).datos;

      const c = cuentas.find((item) => item.id === datos.cuenta_id);
      const cat = categorias.find((item) => item.id === datos.categoria_id);

      const esGasto = datos.tipo.toLowerCase() === 'gasto';
      const [entero, frac = ''] = datos.monto.split('.');
      const montoLimpio = `${entero}.${frac.padEnd(2, '0')}`;

      const nuevoMov: Movimiento = {
        id: `mov-${++contador}`,
        cuenta_id: datos.cuenta_id,
        tipo: datos.tipo,
        monto: montoLimpio,
        monto_con_signo: `${esGasto ? '−' : '+'} ${montoLimpio}`,
        fecha: datos.fecha,
        categoria_id: datos.categoria_id,
        categoria_nombre: cat ? cat.nombre : 'General',
        nota: datos.nota,
        estado: 'normal',
        ajustes: [],
      };

      if (!movimientosMap[datos.cuenta_id]) {
        movimientosMap[datos.cuenta_id] = [];
      }
      movimientosMap[datos.cuenta_id].unshift(nuevoMov);

      // Actualizar saldo de cuenta
      if (c) {
        const saldoCent = aCentavos(c.saldo_actual);
        const montoCent = aCentavos(montoLimpio);
        const nuevoCent = esGasto ? saldoCent - montoCent : saldoCent + montoCent;
        c.saldo_actual = (nuevoCent / 100).toFixed(2);
      }

      return nuevoMov;
    }

    // 9. Corregir movimiento
    if (cmd === 'corregir_movimiento') {
      const datos = (args as { datos: { movimiento_id: string; monto_correcto: string } }).datos;

      // Buscar movimiento
      for (const [cuentaId, lista] of Object.entries(movimientosMap)) {
        const m = lista.find((item) => item.id === datos.movimiento_id);
        if (m) {
          m.estado = 'corregido';
          m.monto_correcto = datos.monto_correcto;

          const ajusteMov: Movimiento = {
            id: `mov-ajuste-${++contador}`,
            cuenta_id: cuentaId,
            tipo: m.tipo,
            monto: datos.monto_correcto,
            monto_con_signo: `Ajuste (${datos.monto_correcto})`,
            fecha: new Date().toISOString().split('T')[0],
            categoria_id: m.categoria_id,
            categoria_nombre: `${m.categoria_nombre} (Ajuste)`,
            nota: `Ajuste del movimiento ${m.id}`,
            estado: 'ajuste',
            corrige_a: m.id,
            ajustes: [],
          };
          lista.unshift(ajusteMov);
          return ajusteMov;
        }
      }

      throw new Error('Movimiento no encontrado');
    }

    throw new Error(`Comando no simulado: ${cmd}`);
  });
}
