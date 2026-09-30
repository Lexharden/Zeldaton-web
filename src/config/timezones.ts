/**
 * Time zones of the Americas offered when configuring a racer. Every id is an IANA name that both
 * the browser (Intl) and the backend (chrono-tz) understand; `backend/tests/timezones.rs` checks it.
 * Each racer's daily budget resets at the event's reset hour in their own zone, and the IANA rules
 * handle daylight saving by themselves (e.g. Mexicali changes hour with California, CDMX does not).
 */
export interface AmericasZone {
  /** IANA id stored on the racer. */
  id: string
  /** Name people recognise: the city or area, not the IANA spelling. */
  city: string
  /** Where it applies, shown next to the city in the picker. */
  area: string
  /** ISO 3166-1 alpha-2, used to prefill the racer's country. */
  country: string
}

export interface ZoneGroup {
  label: string
  zones: AmericasZone[]
}

export const AMERICAS_TIMEZONES: ZoneGroup[] = [
  {
    label: 'México',
    zones: [
      { id: 'America/Tijuana', city: 'Mexicali / Tijuana', area: 'Baja California', country: 'MX' },
      { id: 'America/Hermosillo', city: 'Hermosillo', area: 'Sonora', country: 'MX' },
      {
        id: 'America/Mazatlan',
        city: 'Mazatlán / La Paz',
        area: 'Sinaloa, Nayarit, BCS',
        country: 'MX',
      },
      { id: 'America/Chihuahua', city: 'Chihuahua', area: 'Chihuahua', country: 'MX' },
      {
        id: 'America/Ciudad_Juarez',
        city: 'Ciudad Juárez',
        area: 'Chihuahua (frontera)',
        country: 'MX',
      },
      {
        id: 'America/Mexico_City',
        city: 'Ciudad de México',
        area: 'Centro del país',
        country: 'MX',
      },
      { id: 'America/Monterrey', city: 'Monterrey', area: 'Nuevo León', country: 'MX' },
      { id: 'America/Matamoros', city: 'Matamoros', area: 'Tamaulipas (frontera)', country: 'MX' },
      { id: 'America/Merida', city: 'Mérida', area: 'Yucatán, Campeche', country: 'MX' },
      { id: 'America/Cancun', city: 'Cancún', area: 'Quintana Roo', country: 'MX' },
    ],
  },
  {
    label: 'Estados Unidos y Canadá',
    zones: [
      { id: 'Pacific/Honolulu', city: 'Honolulu', area: 'Hawái', country: 'US' },
      { id: 'America/Anchorage', city: 'Anchorage', area: 'Alaska', country: 'US' },
      { id: 'America/Los_Angeles', city: 'Los Ángeles', area: 'Pacífico (EE. UU.)', country: 'US' },
      { id: 'America/Vancouver', city: 'Vancouver', area: 'Pacífico (Canadá)', country: 'CA' },
      { id: 'America/Phoenix', city: 'Phoenix', area: 'Arizona', country: 'US' },
      { id: 'America/Denver', city: 'Denver', area: 'Montaña (EE. UU.)', country: 'US' },
      { id: 'America/Edmonton', city: 'Edmonton', area: 'Montaña (Canadá)', country: 'CA' },
      { id: 'America/Chicago', city: 'Chicago / Texas', area: 'Centro (EE. UU.)', country: 'US' },
      { id: 'America/Winnipeg', city: 'Winnipeg', area: 'Centro (Canadá)', country: 'CA' },
      { id: 'America/New_York', city: 'Nueva York / Miami', area: 'Este (EE. UU.)', country: 'US' },
      { id: 'America/Toronto', city: 'Toronto', area: 'Este (Canadá)', country: 'CA' },
      { id: 'America/Halifax', city: 'Halifax', area: 'Atlántico (Canadá)', country: 'CA' },
      { id: 'America/St_Johns', city: "St. John's", area: 'Terranova', country: 'CA' },
    ],
  },
  {
    label: 'Centroamérica',
    zones: [
      { id: 'America/Guatemala', city: 'Guatemala', area: 'Guatemala', country: 'GT' },
      { id: 'America/Belize', city: 'Belice', area: 'Belice', country: 'BZ' },
      { id: 'America/El_Salvador', city: 'San Salvador', area: 'El Salvador', country: 'SV' },
      { id: 'America/Tegucigalpa', city: 'Tegucigalpa', area: 'Honduras', country: 'HN' },
      { id: 'America/Managua', city: 'Managua', area: 'Nicaragua', country: 'NI' },
      { id: 'America/Costa_Rica', city: 'San José', area: 'Costa Rica', country: 'CR' },
      { id: 'America/Panama', city: 'Panamá', area: 'Panamá', country: 'PA' },
    ],
  },
  {
    label: 'Caribe',
    zones: [
      { id: 'America/Havana', city: 'La Habana', area: 'Cuba', country: 'CU' },
      { id: 'America/Jamaica', city: 'Kingston', area: 'Jamaica', country: 'JM' },
      { id: 'America/Port-au-Prince', city: 'Puerto Príncipe', area: 'Haití', country: 'HT' },
      {
        id: 'America/Santo_Domingo',
        city: 'Santo Domingo',
        area: 'República Dominicana',
        country: 'DO',
      },
      { id: 'America/Puerto_Rico', city: 'San Juan', area: 'Puerto Rico', country: 'PR' },
    ],
  },
  {
    label: 'Sudamérica',
    zones: [
      { id: 'America/Bogota', city: 'Bogotá', area: 'Colombia', country: 'CO' },
      { id: 'America/Guayaquil', city: 'Quito / Guayaquil', area: 'Ecuador', country: 'EC' },
      { id: 'America/Lima', city: 'Lima', area: 'Perú', country: 'PE' },
      { id: 'America/Caracas', city: 'Caracas', area: 'Venezuela', country: 'VE' },
      { id: 'America/La_Paz', city: 'La Paz', area: 'Bolivia', country: 'BO' },
      { id: 'America/Manaus', city: 'Manaos', area: 'Amazonas (Brasil)', country: 'BR' },
      { id: 'America/Santiago', city: 'Santiago', area: 'Chile', country: 'CL' },
      {
        id: 'America/Punta_Arenas',
        city: 'Punta Arenas',
        area: 'Magallanes (Chile)',
        country: 'CL',
      },
      { id: 'America/Asuncion', city: 'Asunción', area: 'Paraguay', country: 'PY' },
      {
        id: 'America/Argentina/Buenos_Aires',
        city: 'Buenos Aires',
        area: 'Argentina',
        country: 'AR',
      },
      { id: 'America/Montevideo', city: 'Montevideo', area: 'Uruguay', country: 'UY' },
      { id: 'America/Sao_Paulo', city: 'São Paulo / Río', area: 'Brasil (sureste)', country: 'BR' },
      { id: 'America/Guyana', city: 'Georgetown', area: 'Guyana', country: 'GY' },
      { id: 'America/Paramaribo', city: 'Paramaribo', area: 'Surinam', country: 'SR' },
    ],
  },
]

const BY_ID = new Map(AMERICAS_TIMEZONES.flatMap((g) => g.zones).map((z) => [z.id, z]))

export function findZone(id: string | null | undefined): AmericasZone | undefined {
  return id ? BY_ID.get(id) : undefined
}

/** "UTC−7" right now (so it already reflects daylight saving). Empty if the browser does not know the id. */
export function currentOffset(id: string, at: Date = new Date()): string {
  try {
    const part = new Intl.DateTimeFormat('en-US', { timeZone: id, timeZoneName: 'shortOffset' })
      .formatToParts(at)
      .find((p) => p.type === 'timeZoneName')?.value
    if (!part) return ''
    const offset = part.replace('GMT', '')
    return `UTC${offset ? offset.replace('-', '−') : '±0'}`
  } catch {
    return ''
  }
}
