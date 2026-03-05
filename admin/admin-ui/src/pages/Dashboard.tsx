import { useState, useEffect } from "react";
import {
  getGlobalStats,
  getModelBreakdown,
  getTopUsers,
  getRecentMessages,
  GlobalStats,
  ModelBreakdownRow,
  TopUserRow,
  RecentMessageRow,
} from "../api";

function Card({ title, value }: { title: string; value: string | number }) {
  return (
    <div
      style={{
        padding: "1rem 1.5rem",
        background: "#1e293b",
        borderRadius: "8px",
        border: "1px solid #334155",
      }}
    >
      <div style={{ fontSize: "0.75rem", color: "#94a3b8", marginBottom: "0.25rem" }}>{title}</div>
      <div style={{ fontSize: "1.5rem", fontWeight: 600 }}>{value}</div>
    </div>
  );
}

function formatDateTime(value: unknown): string {
  if (value == null) return "";
  const str = String(value);
  try {
    const d = new Date(str);
    if (isNaN(d.getTime())) return str;
    return d.toLocaleString(undefined, {
      dateStyle: "medium",
      timeStyle: "medium",
    });
  } catch {
    return str;
  }
}

type TableColumn<T> = { key: keyof T; label: string; format?: (v: unknown) => string };

function Table<T extends object>({
  columns,
  data,
  rowKey,
}: {
  columns: TableColumn<T>[];
  data: T[];
  rowKey: (row: T) => string;
}) {
  return (
    <table style={{ width: "100%", borderCollapse: "collapse" }}>
      <thead>
        <tr style={{ borderBottom: "1px solid #334155" }}>
          {columns.map((c) => (
            <th key={String(c.key)} style={{ textAlign: "left", padding: "0.75rem", fontSize: "0.75rem", color: "#94a3b8" }}>
              {c.label}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {data.map((row) => (
          <tr key={rowKey(row)} style={{ borderBottom: "1px solid #1e293b" }}>
            {columns.map((c) => {
              const raw = (row as Record<string, unknown>)[c.key as string];
              const display = c.format ? c.format(raw) : String(raw ?? "");
              return (
                <td key={String(c.key)} style={{ padding: "0.75rem", fontSize: "0.875rem" }}>
                  {display}
                </td>
              );
            })}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export default function Dashboard() {
  const [stats, setStats] = useState<GlobalStats | null>(null);
  const [modelBreakdown, setModelBreakdown] = useState<ModelBreakdownRow[]>([]);
  const [topUsers, setTopUsers] = useState<TopUserRow[]>([]);
  const [recentMessages, setRecentMessages] = useState<RecentMessageRow[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      try {
        const [s, m, t, r] = await Promise.all([
          getGlobalStats(),
          getModelBreakdown(),
          getTopUsers(),
          getRecentMessages(),
        ]);
        setStats(s);
        setModelBreakdown(m);
        setTopUsers(t);
        setRecentMessages(r);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load");
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  const handleLogout = () => {
    localStorage.removeItem("admin_token");
    window.location.href = "/login";
  };

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center" }}>Loading...</div>
    );
  }

  if (error) {
    return (
      <div style={{ padding: "2rem", color: "#f87171" }}>
        {error}
        <button onClick={() => window.location.reload()} style={{ marginLeft: "1rem" }}>
          Retry
        </button>
      </div>
    );
  }

  return (
    <div style={{ padding: "2rem", maxWidth: "1200px", margin: "0 auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem" }}>
        <h1 style={{ margin: 0, fontSize: "1.5rem" }}>Ghost Admin Dashboard</h1>
        <button
          onClick={handleLogout}
          style={{
            padding: "0.5rem 1rem",
            background: "transparent",
            border: "1px solid #475569",
            borderRadius: "4px",
            color: "#94a3b8",
            cursor: "pointer",
          }}
        >
          Logout
        </button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "1rem", marginBottom: "2rem" }}>
        <Card title="Total Users" value={stats?.total_users ?? 0} />
        <Card title="Total Tokens" value={stats?.total_tokens?.toLocaleString() ?? 0} />
        <Card title="Total Cost (USD)" value={stats?.total_cost_usd ?? "0"} />
        <Card title="Total Revenue (USD)" value={stats?.total_revenue ?? "0"} />
      </div>

      <section style={{ marginBottom: "2rem" }}>
        <h2 style={{ fontSize: "1rem", marginBottom: "1rem" }}>Model Breakdown</h2>
        <div style={{ background: "#1e293b", borderRadius: "8px", overflow: "hidden", border: "1px solid #334155" }}>
          <Table
            columns={[
              { key: "model", label: "Model" },
              { key: "provider", label: "Provider" },
              { key: "tokens", label: "Tokens" },
              { key: "cost_usd", label: "Cost USD" },
              { key: "requests", label: "Requests" },
            ]}
            data={modelBreakdown}
            rowKey={(r) => `${r.model}-${r.provider}`}
          />
        </div>
      </section>

      <section style={{ marginBottom: "2rem" }}>
        <h2 style={{ fontSize: "1rem", marginBottom: "1rem" }}>Top Users</h2>
        <div style={{ background: "#1e293b", borderRadius: "8px", overflow: "hidden", border: "1px solid #334155" }}>
          <Table
            columns={[
              { key: "email", label: "Email" },
              { key: "tokens", label: "Tokens" },
              { key: "cost_usd", label: "Cost USD" },
            ]}
            data={topUsers}
            rowKey={(r) => `${r.email ?? "null"}-${r.tokens}`}
          />
        </div>
      </section>

      <section>
        <h2 style={{ fontSize: "1rem", marginBottom: "1rem" }}>Recent Messages</h2>
        <div style={{ background: "#1e293b", borderRadius: "8px", overflow: "hidden", border: "1px solid #334155", maxHeight: "400px", overflowY: "auto" }}>
          <Table
            columns={[
              { key: "email", label: "User" },
              { key: "model", label: "Model" },
              { key: "provider", label: "Provider" },
              { key: "total_tokens", label: "Tokens" },
              { key: "cost_usd", label: "Cost" },
              { key: "created_at", label: "Date & Time", format: formatDateTime },
            ]}
            data={recentMessages}
            rowKey={(r) => `${r.user_id}-${r.created_at}`}
          />
        </div>
      </section>
    </div>
  );
}
