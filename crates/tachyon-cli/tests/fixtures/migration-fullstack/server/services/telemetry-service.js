// @ts-check

import FyloTelemetryRepository from '../repositories/fylo-telemetry-repository.js';

class OtlpValueDecoder {
    /**
     * @param {Record<string, unknown>} value
     * @returns {unknown}
     */
    static decode(value) {
        if ('stringValue' in value) return value.stringValue;
        if ('boolValue' in value) return value.boolValue;
        if ('intValue' in value) return Number(value.intValue);
        if ('doubleValue' in value) return value.doubleValue;
        if ('arrayValue' in value) {
            const entries = /** @type {{ values?: Array<Record<string, unknown>> }} */ (value.arrayValue);
            return (entries.values ?? []).map((entry) => OtlpValueDecoder.decode(entry));
        }
        if ('kvlistValue' in value) {
            const entries = /** @type {{ values?: Array<{ key: string, value: Record<string, unknown> }> }} */ (value.kvlistValue);
            const decodedPairs = (entries.values ?? []).map((entry) => [entry.key, OtlpValueDecoder.decode(entry.value)]);
            return Object.fromEntries(decodedPairs);
        }
        if ('bytesValue' in value) return value.bytesValue;
        return undefined;
    }
}

export default class TelemetryService {
    /**
     * @param {string | undefined} value
     * @returns {string | undefined}
     */
    static environmentPath(value) {
        return value && value.trim() ? value : undefined;
    }

    /**
     * @param {{ repository?: FyloTelemetryRepository, collection?: string, root?: string }} [options]
     */
    constructor(options = {}) {
        const root = TelemetryService.environmentPath(options.root)
            ?? TelemetryService.environmentPath(process.env.YON_OTEL_ROOT)
            ?? TelemetryService.environmentPath(process.env.FYLO_ROOT)
            ?? `${process.cwd()}/db`;
        const collection = options.collection ?? 'otel-spans';
        this.repository = options.repository ?? new FyloTelemetryRepository({ root, collection });
        this.collection = collection;
    }

    /**
     * @param {{ query?: Record<string, unknown> }} request
     * @returns {Promise<{ summary: Record<string, unknown>, recent: Array<Record<string, unknown>> }>}
     */
    async readFromRequest(request) {
        return await this.read(TelemetryService.limitFromQuery(request.query));
    }

    /**
     * @param {number} limit
     * @returns {Promise<{ summary: Record<string, unknown>, recent: Array<Record<string, unknown>> }>}
     */
    async read(limit) {
        const spans = await this.loadSpans();
        const sorted = spans.sort((left, right) => {
            const leftStart = Number(left.startTimeUnixNano ?? 0);
            const rightStart = Number(right.startTimeUnixNano ?? 0);
            return rightStart - leftStart;
        });
        const telemetryEnabled = process.env.YON_OTEL_ENABLED === 'true';
        const dataBrowserEnabled = process.env.YON_DATA_BROWSER_ENABLED === 'true';
        return {
            summary: {
                enabled: telemetryEnabled || dataBrowserEnabled,
                collection: this.collection,
                spanCount: spans.length,
                requestCount: spans.filter((span) => span.kind === 'server').length,
                errorCount: spans.filter((span) => Number(span.statusCode) >= 500).length,
            },
            recent: sorted.slice(0, limit),
        };
    }

    /**
     * @returns {Promise<Array<Record<string, unknown>>>}
     */
    async loadSpans() {
        const flatSpans = await this.repository.queryFlatSpans();
        if (flatSpans.length > 0) {
            return flatSpans.map((flatSpan) => this.normalizeFlatSpan(flatSpan));
        }

        /** @type {Array<Record<string, unknown>>} */
        const spans = [];
        for (const entry of await this.repository.findPersistedEntries()) {
            if (typeof entry.otlpJson === 'string') {
                spans.push(...this.extractPersistedSpans(entry));
            }
            else {
                spans.push(this.normalizeFlatSpan(entry));
            }
        }
        return spans;
    }

    /**
     * @param {Record<string, unknown>} flatSpan
     * @returns {Record<string, unknown>}
     */
    normalizeFlatSpan(flatSpan) {
        const durationMs = TelemetryService.flatSpanDuration(flatSpan);
        return {
            traceId: flatSpan.traceId ?? '',
            spanId: flatSpan.spanId ?? '',
            parentSpanId: flatSpan.parentSpanId ?? '',
            kind: flatSpan.kind ?? 'server',
            name: flatSpan.name ?? '',
            requestId: flatSpan.requestId ?? null,
            route: flatSpan.route ?? null,
            method: flatSpan.method ?? null,
            statusCode: Number(flatSpan.statusCode) || null,
            traceState: flatSpan.traceState ?? '',
            startTimeUnixNano: flatSpan.startTimeUnixNano ?? '',
            endTimeUnixNano: flatSpan.endTimeUnixNano ?? '',
            durationMs,
        };
    }

    /**
     * @param {Record<string, unknown>} flatSpan
     * @returns {number}
     */
    static flatSpanDuration(flatSpan) {
        const startNanoseconds = Number(flatSpan.startTimeUnixNano ?? 0);
        const endNanoseconds = Number(flatSpan.endTimeUnixNano ?? 0);
        if (startNanoseconds && endNanoseconds) {
            return Math.max(0, (endNanoseconds - startNanoseconds) / 1_000_000);
        }
        return Number(flatSpan.durationMs) || 0;
    }

    /**
     * @param {Record<string, unknown>} persistedEntry
     * @returns {Array<Record<string, unknown>>}
     */
    extractPersistedSpans(persistedEntry) {
        if (typeof persistedEntry.otlpJson !== 'string') return [];
        let tracesData;
        try {
            tracesData = JSON.parse(persistedEntry.otlpJson);
        } catch {
            return [];
        }

        /** @type {Array<Record<string, unknown>>} */
        const spans = [];
        for (const resourceSpan of tracesData.resourceSpans ?? []) {
            for (const scopeSpan of resourceSpan.scopeSpans ?? []) {
                for (const span of scopeSpan.spans ?? []) {
                    spans.push(this.normalizeSpan(span));
                }
            }
        }
        return spans;
    }

    /**
     * @param {Record<string, unknown>} span
     * @returns {Record<string, unknown>}
     */
    normalizeSpan(span) {
        const startTimeUnixNano = String(span.startTimeUnixNano ?? '0');
        const endTimeUnixNano = String(span.endTimeUnixNano ?? '0');
        const elapsedNanoseconds = Number(endTimeUnixNano) - Number(startTimeUnixNano);
        const durationMs = Math.max(0, elapsedNanoseconds / 1_000_000);
        return {
            traceId: span.traceId ?? '',
            spanId: span.spanId ?? '',
            parentSpanId: span.parentSpanId ?? '',
            kind: TelemetryService.kindLabel(Number(span.kind)),
            name: span.name ?? '',
            requestId: this.attribute(span, 'tachyon.request.id') ?? null,
            route: this.attribute(span, 'http.route') ?? null,
            method: this.attribute(span, 'http.request.method') ?? null,
            statusCode: this.attribute(span, 'http.response.status_code') ?? null,
            traceState: span.traceState ?? '',
            startTimeUnixNano,
            endTimeUnixNano,
            durationMs,
        };
    }

    /**
     * @param {Record<string, unknown>} span
     * @param {string} key
     * @returns {unknown}
     */
    attribute(span, key) {
        const attributes = /** @type {Array<{ key: string, value: Record<string, unknown> }>} */ (span.attributes ?? []);
        const match = attributes.find((entry) => entry.key === key);
        return match ? OtlpValueDecoder.decode(match.value) : undefined;
    }

    /**
     * @param {Record<string, unknown> | undefined} query
     * @returns {number}
     */
    static limitFromQuery(query) {
        const limitValue = Number(query?.limit ?? 12);
        if (!Number.isFinite(limitValue) || limitValue <= 0) {
            return 12;
        }
        return Math.min(Math.trunc(limitValue), 50);
    }

    /**
     * @param {number} kind
     * @returns {string}
     */
    static kindLabel(kind) {
        if (kind === 2) return 'server';
        if (kind === 1) return 'internal';
        if (kind === 3) return 'client';
        if (kind === 4) return 'producer';
        if (kind === 5) return 'consumer';
        return 'unspecified';
    }
}
