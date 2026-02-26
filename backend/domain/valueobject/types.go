package valueobject

import "time"

// TimeRange represents a bounded time period for queries
type TimeRange struct {
	From time.Time
	To   time.Time
}

func NewTimeRange(from, to time.Time) TimeRange {
	if to.Before(from) {
		from, to = to, from
	}
	return TimeRange{From: from, To: to}
}

func Last24Hours() TimeRange {
	now := time.Now()
	return TimeRange{From: now.Add(-24 * time.Hour), To: now}
}

func Last7Days() TimeRange {
	now := time.Now()
	return TimeRange{From: now.Add(-7 * 24 * time.Hour), To: now}
}

// Pagination for list queries
type Pagination struct {
	Limit  int
	Offset int
}

func NewPagination(limit, offset int) Pagination {
	if limit <= 0 || limit > 10000 {
		limit = 100
	}
	if offset < 0 {
		offset = 0
	}
	return Pagination{Limit: limit, Offset: offset}
}

// AggregationInterval for time-bucketed queries
type AggregationInterval string

const (
	Interval1Min  AggregationInterval = "1 minute"
	Interval5Min  AggregationInterval = "5 minutes"
	Interval15Min AggregationInterval = "15 minutes"
	Interval30Min AggregationInterval = "30 minutes"
	Interval1Hour AggregationInterval = "1 hour"
	Interval6Hour AggregationInterval = "6 hours"
	Interval1Day  AggregationInterval = "1 day"
	Interval1Week AggregationInterval = "1 week"
)

func (a AggregationInterval) IsValid() bool {
	switch a {
	case Interval1Min, Interval5Min, Interval15Min, Interval30Min,
		Interval1Hour, Interval6Hour, Interval1Day, Interval1Week:
		return true
	}
	return false
}

// AggregatedReading holds bucketed statistics
type AggregatedReading struct {
	Timestamp   time.Time
	AvgValue    float64
	MinValue    float64
	MaxValue    float64
	SampleCount int64
}

// MetricSummary for reporting
type MetricSummary struct {
	Avg     float64
	Min     float64
	Max     float64
	StdDev  float64
	Samples int64
}

// AlarmCounts for dashboard
type AlarmCounts struct {
	Critical int
	Warning  int
	Info     int
}

func (c AlarmCounts) Total() int {
	return c.Critical + c.Warning + c.Info
}

// TelemetryData from MQTT
type TelemetryData struct {
	DeviceID  string
	Timestamp time.Time
	Metrics   map[string]float64
	Quality   int
}

// CommandRequest from API/gRPC
type CommandRequest struct {
	DeviceID   string
	Action     string
	Parameters map[string]string
	IssuedBy   string
}
