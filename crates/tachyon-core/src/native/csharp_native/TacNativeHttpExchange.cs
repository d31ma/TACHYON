internal sealed class TacNativeHttpExchange
{
    private const int ReadSize = 16384;
    private const int AnswerLimit = 1048576;
    private static readonly int[] RedirectStatuses = { 301, 302, 303, 307, 308 };
    private static readonly UTF8Encoding StrictText = new(false, true);
    private readonly System.Threading.CancellationTokenSource cancellation = new();
    private readonly System.Threading.CancellationTokenSource deadline = new();
    private readonly Action finished;
    private string stopReason;

    internal System.Threading.Tasks.Task<string> Answer { get; private set; }

    private TacNativeHttpExchange(Action finished) { this.finished = finished; }

    internal static TacNativeHttpExchange Begin(System.Net.Http.HttpClient client, TacNativeHttpRequest request, Action finished)
    {
        var exchange = new TacNativeHttpExchange(finished);
        exchange.Answer = exchange.Run(client, request);
        return exchange;
    }

    internal void Cancel() => Stop(cancellation, "CANCELLED");

    private void Stop(System.Threading.CancellationTokenSource source, string reason)
    {
        System.Threading.Interlocked.CompareExchange(ref stopReason, reason, null);
        source.Cancel();
    }

    private async System.Threading.Tasks.Task<string> Run(System.Net.Http.HttpClient client, TacNativeHttpRequest request)
    {
        using var watch = deadline.Token.Register(() => System.Threading.Interlocked.CompareExchange(ref stopReason, "DEADLINE", null));
        using var linked = System.Threading.CancellationTokenSource.CreateLinkedTokenSource(cancellation.Token, deadline.Token);
        try
        {
            StartDeadline(request.Deadline);
            var answer = "{\"value\":" + await Exchange(client, request, linked.Token) + "}";
            if (Encoding.UTF8.GetByteCount(answer) > AnswerLimit) throw new TacNativeHttpFailure("HTTP_LIMIT");
            return answer;
        }
        catch (Exception error) { return Failure(error); }
        finally
        {
            deadline.Dispose();
            finished();
        }
    }

    private void StartDeadline(long deadlineTime)
    {
        var remaining = deadlineTime - DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        if (remaining > 0)
        {
            deadline.CancelAfter(TimeSpan.FromMilliseconds(remaining));
            return;
        }
        Stop(deadline, "DEADLINE");
        throw new TacNativeHttpFailure("HTTP_REQUEST");
    }

    private string Failure(Exception error)
    {
        var reason = System.Threading.Volatile.Read(ref stopReason);
        if (reason != null) return TacNativeHttpFailure.Answer(reason);
        if (error is TacNativeHttpFailure failure) return failure.Envelope();
        return TacNativeHttpFailure.Answer("HTTP_NETWORK");
    }

    private static async System.Threading.Tasks.Task<string> Exchange(System.Net.Http.HttpClient client, TacNativeHttpRequest request, System.Threading.CancellationToken stop)
    {
        using var message = request.Message();
        using var response = await client.SendAsync(message, System.Net.Http.HttpCompletionOption.ResponseHeadersRead, stop);
        var status = (int)response.StatusCode;
        if (Array.IndexOf(RedirectStatuses, status) >= 0) throw new TacNativeHttpFailure("HTTP_NETWORK");
        if (status < 200 || status > 299) throw new TacNativeHttpFailure("HTTP_STATUS", status);
        var text = await ReadText(response.Content, request.Limit, stop);
        stop.ThrowIfCancellationRequested();
        if (string.IsNullOrWhiteSpace(text)) return "null";
        var value = ParseValue(text);
        TacNativeJson.Check(value);
        return value?.ToJsonString() ?? "null";
    }

    private static async System.Threading.Tasks.Task<string> ReadText(System.Net.Http.HttpContent content, int limit, System.Threading.CancellationToken stop)
    {
        if (content.Headers.ContentLength > limit) throw new TacNativeHttpFailure("HTTP_LIMIT");
        await using var stream = await content.ReadAsStreamAsync(stop);
        var bytes = await ReadBytes(stream, limit, stop);
        return DecodeText(bytes);
    }

    private static async System.Threading.Tasks.Task<byte[]> ReadBytes(System.IO.Stream stream, int limit, System.Threading.CancellationToken stop)
    {
        using var collected = new System.IO.MemoryStream();
        var buffer = new byte[ReadSize];
        while (true)
        {
            var read = await stream.ReadAsync(buffer, stop);
            if (read == 0) return collected.ToArray();
            if (collected.Length + read > limit) throw new TacNativeHttpFailure("HTTP_LIMIT");
            collected.Write(buffer, 0, read);
        }
    }

    private static string DecodeText(byte[] bytes)
    {
        string text;
        try { text = StrictText.GetString(bytes); }
        catch (DecoderFallbackException) { throw new TacNativeHttpFailure("HTTP_RESPONSE"); }
        return text.StartsWith('﻿') ? text[1..] : text;
    }

    private static JsonNode ParseValue(string text)
    {
        try { return JsonNode.Parse(text); }
        catch (System.Text.Json.JsonException) { throw new TacNativeHttpFailure("HTTP_RESPONSE"); }
    }
}
