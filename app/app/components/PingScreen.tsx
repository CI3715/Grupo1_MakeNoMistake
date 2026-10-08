'use client';

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface PingResponse {
  exito: boolean;
  mensaje: string;
  version: string | null;
  timestamp: string | null;
  latencia_ms: number;
  error: string | null;
}

export default function PingScreen() {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [data, setData] = useState<PingResponse | null>(null);

  const conectado = data !== null && !error && !loading;

  const handleVerificarConexion = async () => {
    setLoading(true);
    setError(null);
    setData(null);

    try {
      const response = await invoke<PingResponse>('ping_servidor');
      if (response.exito) {
        setData(response);
      } else {
        setError(response.mensaje);
      }
    } catch (err: unknown) {
      if (typeof err === 'string') {
        setError(err);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Error desconocido al intentar conectar con el servidor.');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="w-full max-w-lg mx-auto p-6 bg-white rounded-lg shadow-md border border-gray-200">
      <h2 className="text-xl font-bold mb-4 text-gray-800">
        Estado del Servidor
      </h2>

      <div className="flex items-center justify-between mb-4">
        <span className="font-medium text-gray-700">Estado:</span>
        {loading ? (
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold bg-amber-100 text-amber-800">
            Verificando...
          </span>
        ) : conectado ? (
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold bg-green-100 text-green-800">
            Conectado
          </span>
        ) : (
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold bg-gray-100 text-gray-700">
            Desconectado
          </span>
        )}
      </div>

      <button
        onClick={handleVerificarConexion}
        disabled={loading}
        className="w-full py-2.5 px-4 bg-indigo-600 hover:bg-indigo-700 text-white font-medium rounded-md transition duration-150 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      >
        {loading ? (
          <>
            <span className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></span>
            Verificando...
          </>
        ) : (
          'Verificar Conexión con el Servidor'
        )}
      </button>

      <div className="mt-6 space-y-4">
        {error && (
          <div className="p-4 bg-red-50 border border-red-200 text-red-700 rounded-md text-sm">
            <p className="font-semibold">Error de Conexión:</p>
            <p>{error}</p>
          </div>
        )}

        {conectado && data && (
          <div className="p-4 bg-green-50 border border-green-200 text-gray-800 rounded-md text-sm">
            <div className="grid grid-cols-2 gap-2">
              <div>
                <span className="text-gray-500 block text-xs">Mensaje:</span>
                <span className="font-medium">{data.mensaje}</span>
              </div>
              <div>
                <span className="text-gray-500 block text-xs">Versión:</span>
                <span className="font-medium">{data.version}</span>
              </div>
              <div className="col-span-2">
                <span className="text-gray-500 block text-xs">Latencia:</span>
                <span className="font-medium">{data.latencia_ms} ms</span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
