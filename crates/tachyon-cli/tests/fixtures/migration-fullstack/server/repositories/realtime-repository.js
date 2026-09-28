// @ts-check
import { Fylo } from '../../../../../src/vendor/fylo/fylo-node.mjs';
import { fyloOptions } from '../../../../../src/server/fylo-options.js';
import YonRealtime from '../../../../../src/server/realtime/realtime.js';
import TopicLog from '../../../../../src/server/realtime/topic-log.js';

export default class RealtimeRepository {
    static clientIdPattern = /^[0-9A-Z]{11}$/;
    static clientsCollection = 'demo-realtime-clients';
    static messagesCollection = 'demo-realtime-messages';

    /** @param {{ root?: string }} [options] */
    constructor(options = {}) {
        this.root = options.root || YonRealtime.root();
        this.fylo = new Fylo(this.root, fyloOptions(this.root));
        this.queue = new TopicLog(this.root);
    }

    /**
     * @param {string} name
     * @returns {{ create(): Promise<void>, put(data: object): Promise<string>, find(query: object): Promise<Record<string, unknown>> }}
     */
    collection(name) {
        return /** @type {Record<string, any>} */ (/** @type {unknown} */ (this.fylo))[name];
    }

    /**
     * @param {{ nickname: string }} input
     * @returns {Promise<{ clientId: string, nickname: string, streamUrl: string, registeredAt: string }>}
     */
    async registerClient(input) {
        await this.collection(RealtimeRepository.clientsCollection).create();
        const clientId = YonRealtime.generateClientId();
        const registeredAt = new Date().toISOString();
        const client = {
            clientId,
            nickname: input.nickname,
            streamUrl: `${this.realtimePath()}/stream?clientId=${clientId}`,
            registeredAt,
        };
        await this.collection(RealtimeRepository.clientsCollection).put(client);
        return client;
    }

    /**
     * @returns {Promise<Array<{ clientId: string, nickname: string, streamUrl: string, registeredAt: string }>>}
     */
    async listClients() {
        /** @type {Array<{ clientId: string, nickname: string, streamUrl: string, registeredAt: string }>} */
        const clients = [];
        try {
            const found = await this.collection(RealtimeRepository.clientsCollection).find({});
            RealtimeRepository.collectClients(found ?? {}, clients);
        }
        catch {
        }
        /** @type {Map<string, { clientId: string, nickname: string, streamUrl: string, registeredAt: string }>} */
        const clientsById = new Map();
        for (const client of clients) {
            clientsById.set(client.clientId, client);
        }
        const uniqueClients = [...clientsById.values()];
        return uniqueClients.sort((left, right) => left.nickname.localeCompare(right.nickname));
    }

    /**
     * @param {{ from: string, to: string, text: string }} input
     * @returns {Promise<{ id: string, to: string, stored: true, event: string, message: { from: string, to: string, text: string, sentAt: string } }>}
     */
    async sendMessage(input) {
        this.assertClientId(input.from, 'from');
        this.assertClientId(input.to, 'to');
        const message = {
            from: input.from,
            to: input.to,
            text: input.text,
            sentAt: new Date().toISOString(),
        };
        const event = 'demo.message';
        const id = await this.queue.publish(this.topic(input.to), {
            to: input.to,
            from: input.from,
            event,
            data: message,
            sentAt: message.sentAt,
        });
        await this.collection(RealtimeRepository.messagesCollection).create();
        await this.collection(RealtimeRepository.messagesCollection).put({
            queueMessageId: id,
            event,
            ...message,
        });
        return { id, to: input.to, stored: true, event, message };
    }

    /** @returns {string} */
    realtimePath() {
        return YonRealtime.publicPath();
    }

    /** @param {string} clientId */
    topic(clientId) {
        this.assertClientId(clientId, 'clientId');
        return `yon.realtime.${clientId}`;
    }

    /**
     * @param {string} value
     * @param {string} field
     */
    assertClientId(value, field) {
        if (!RealtimeRepository.clientIdPattern.test(value)) {
            throw new Error(`${field} must be an 11-character FYLO TTID`);
        }
    }

    /**
     * @param {unknown} value
     * @returns {value is { clientId: string, nickname: string, streamUrl: string, registeredAt: string }}
     */
    static isClient(value) {
        return Boolean(
            value
            && typeof value === 'object'
            && typeof /** @type {{ clientId?: unknown }} */ (value).clientId === 'string'
            && typeof /** @type {{ nickname?: unknown }} */ (value).nickname === 'string'
            && typeof /** @type {{ streamUrl?: unknown }} */ (value).streamUrl === 'string'
            && typeof /** @type {{ registeredAt?: unknown }} */ (value).registeredAt === 'string',
        );
    }

    /**
     * @param {unknown} value
     * @param {Array<{ clientId: string, nickname: string, streamUrl: string, registeredAt: string }>} clients
     */
    static collectClients(value, clients) {
        if (RealtimeRepository.isClient(value)) {
            clients.push(value);
            return;
        }
        if (!value || typeof value !== 'object') return;
        for (const nested of Object.values(/** @type {Record<string, unknown>} */ (value))) {
            RealtimeRepository.collectClients(nested, clients);
        }
    }
}
