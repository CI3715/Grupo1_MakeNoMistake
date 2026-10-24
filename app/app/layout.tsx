import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { SCRIPT_APARIENCIA } from "./utils/apariencia";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Cuentas Claras",
  description: "Cuentas Claras: tus cuentas, guardadas en tu dispositivo",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    // suppressHydrationWarning: el script de abajo escribe data-tema y data-paleta
    // en <html> antes de que React hidrate.
    <html
      lang="es"
      suppressHydrationWarning
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <head>
        {/* Aplica el tema guardado antes del primer pintado para evitar parpadeos */}
        <script dangerouslySetInnerHTML={{ __html: SCRIPT_APARIENCIA }} />
      </head>
      <body className="min-h-full flex flex-col bg-fondo text-texto selection:bg-acento/20">
        {children}
      </body>
    </html>
  );
}
