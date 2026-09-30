import type { Catalog } from '@/types/catalog'
import factory from '../../contract/rest/catalog-default.json'

/**
 * The factory catalog, used until (or instead of) `/api/catalog`: mock mode, first paint and a
 * backend that is down. It is the server's own output (backend/tests/contract.rs keeps the file in
 * sync), so the website and the server can never disagree about the factory ids.
 */
export const DEFAULT_CATALOG = factory as Catalog
