package dds

import (
	"encoding/binary"
	"time"
)

// ═══════════════════════════════════════════════
// QoS Policy Definitions for DDS/RTPS
// ═══════════════════════════════════════════════

// ReliabilityKind defines reliability semantics
type ReliabilityKind uint32

const (
	ReliabilityBestEffort ReliabilityKind = 1
	ReliabilityReliable   ReliabilityKind = 2
)

func (r ReliabilityKind) String() string {
	switch r {
	case ReliabilityBestEffort:
		return "BEST_EFFORT"
	case ReliabilityReliable:
		return "RELIABLE"
	default:
		return "UNKNOWN"
	}
}

// DurabilityKind defines data persistence semantics
type DurabilityKind uint32

const (
	DurabilityVolatile        DurabilityKind = 0
	DurabilityTransientLocal  DurabilityKind = 1
	DurabilityTransient       DurabilityKind = 2
	DurabilityPersistent      DurabilityKind = 3
)

func (d DurabilityKind) String() string {
	switch d {
	case DurabilityVolatile:
		return "VOLATILE"
	case DurabilityTransientLocal:
		return "TRANSIENT_LOCAL"
	case DurabilityTransient:
		return "TRANSIENT"
	case DurabilityPersistent:
		return "PERSISTENT"
	default:
		return "UNKNOWN"
	}
}

// HistoryKind defines how many samples to store
type HistoryKind uint32

const (
	HistoryKeepLast HistoryKind = 0
	HistoryKeepAll  HistoryKind = 1
)

func (h HistoryKind) String() string {
	switch h {
	case HistoryKeepLast:
		return "KEEP_LAST"
	case HistoryKeepAll:
		return "KEEP_ALL"
	default:
		return "UNKNOWN"
	}
}

// OwnershipKind defines data ownership semantics
type OwnershipKind uint32

const (
	OwnershipShared    OwnershipKind = 0
	OwnershipExclusive OwnershipKind = 1
)

func (o OwnershipKind) String() string {
	switch o {
	case OwnershipShared:
		return "SHARED"
	case OwnershipExclusive:
		return "EXCLUSIVE"
	default:
		return "UNKNOWN"
	}
}

// DDSDuration represents a DDS duration (seconds + fraction)
type DDSDuration struct {
	Seconds  int32  `json:"seconds"`
	Fraction uint32 `json:"fraction"`
}

// DurationInfinite represents an infinite DDS duration
var DurationInfinite = DDSDuration{Seconds: 0x7FFFFFFF, Fraction: 0xFFFFFFFF}

// DurationZero represents a zero DDS duration
var DurationZero = DDSDuration{Seconds: 0, Fraction: 0}

// FromGoDuration converts a Go time.Duration to a DDSDuration
func FromGoDuration(d time.Duration) DDSDuration {
	sec := int32(d / time.Second)
	nsec := uint32((d % time.Second).Nanoseconds())
	// Convert nanoseconds to DDS fraction (nanoseconds * 2^32 / 10^9)
	fraction := uint32(float64(nsec) * 4.294967296)
	return DDSDuration{Seconds: sec, Fraction: fraction}
}

// ToGoDuration converts a DDSDuration to Go time.Duration
func (d DDSDuration) ToGoDuration() time.Duration {
	if d.Seconds == 0x7FFFFFFF && d.Fraction == 0xFFFFFFFF {
		return time.Duration(1<<63 - 1) // MaxInt64 duration
	}
	nsec := float64(d.Fraction) / 4.294967296
	return time.Duration(d.Seconds)*time.Second + time.Duration(nsec)*time.Nanosecond
}

// IsInfinite returns true if this duration represents infinity
func (d DDSDuration) IsInfinite() bool {
	return d.Seconds == 0x7FFFFFFF && d.Fraction == 0xFFFFFFFF
}

// QoSProfile bundles all QoS policies for a DDS endpoint
type QoSProfile struct {
	Reliability  ReliabilityPolicy  `json:"reliability"`
	Durability   DurabilityPolicy   `json:"durability"`
	Deadline     DeadlinePolicy     `json:"deadline"`
	Lifespan     LifespanPolicy     `json:"lifespan"`
	History      HistoryPolicy      `json:"history"`
	Ownership    OwnershipPolicy    `json:"ownership"`
	Partition    PartitionPolicy    `json:"partition"`
}

// ReliabilityPolicy controls whether data delivery is guaranteed
type ReliabilityPolicy struct {
	Kind            ReliabilityKind `json:"kind"`
	MaxBlockingTime DDSDuration     `json:"max_blocking_time"`
}

// DurabilityPolicy controls whether late-joining readers can get historical data
type DurabilityPolicy struct {
	Kind DurabilityKind `json:"kind"`
}

// DeadlinePolicy defines the maximum expected interval between data publications
type DeadlinePolicy struct {
	Period DDSDuration `json:"period"`
}

// LifespanPolicy defines how long data remains valid after publication
type LifespanPolicy struct {
	Duration DDSDuration `json:"duration"`
}

// HistoryPolicy controls how many samples are kept in the reader/writer cache
type HistoryPolicy struct {
	Kind  HistoryKind `json:"kind"`
	Depth int32       `json:"depth"`
}

// OwnershipPolicy controls which writer's data takes precedence
type OwnershipPolicy struct {
	Kind     OwnershipKind `json:"kind"`
	Strength int32         `json:"strength"`
}

// PartitionPolicy groups endpoints into logical partitions
type PartitionPolicy struct {
	Names []string `json:"names"`
}

// ═══════════════════════════════════════════════
// Pre-defined QoS Profiles for SCADA Topics
// ═══════════════════════════════════════════════

// QoSSensorData returns QoS for high-frequency sensor data streams.
// Best-effort reliability, volatile durability, short deadline.
func QoSSensorData() QoSProfile {
	return QoSProfile{
		Reliability: ReliabilityPolicy{
			Kind:            ReliabilityBestEffort,
			MaxBlockingTime: FromGoDuration(100 * time.Millisecond),
		},
		Durability: DurabilityPolicy{Kind: DurabilityVolatile},
		Deadline:   DeadlinePolicy{Period: FromGoDuration(1 * time.Second)},
		Lifespan:   LifespanPolicy{Duration: FromGoDuration(5 * time.Second)},
		History: HistoryPolicy{
			Kind:  HistoryKeepLast,
			Depth: 1,
		},
		Ownership: OwnershipPolicy{Kind: OwnershipShared},
		Partition: PartitionPolicy{Names: []string{"scada"}},
	}
}

// QoSAlarm returns QoS for alarm events.
// Reliable delivery, transient-local durability so late joiners get recent alarms.
func QoSAlarm() QoSProfile {
	return QoSProfile{
		Reliability: ReliabilityPolicy{
			Kind:            ReliabilityReliable,
			MaxBlockingTime: FromGoDuration(5 * time.Second),
		},
		Durability: DurabilityPolicy{Kind: DurabilityTransientLocal},
		Deadline:   DeadlinePolicy{Period: DurationInfinite},
		Lifespan:   LifespanPolicy{Duration: FromGoDuration(1 * time.Hour)},
		History: HistoryPolicy{
			Kind:  HistoryKeepAll,
			Depth: 100,
		},
		Ownership: OwnershipPolicy{Kind: OwnershipShared},
		Partition: PartitionPolicy{Names: []string{"scada"}},
	}
}

// QoSHeartbeat returns QoS for periodic heartbeat messages.
// Best-effort, volatile, with a strict deadline.
func QoSHeartbeat() QoSProfile {
	return QoSProfile{
		Reliability: ReliabilityPolicy{
			Kind:            ReliabilityBestEffort,
			MaxBlockingTime: FromGoDuration(100 * time.Millisecond),
		},
		Durability: DurabilityPolicy{Kind: DurabilityVolatile},
		Deadline:   DeadlinePolicy{Period: FromGoDuration(10 * time.Second)},
		Lifespan:   LifespanPolicy{Duration: FromGoDuration(30 * time.Second)},
		History: HistoryPolicy{
			Kind:  HistoryKeepLast,
			Depth: 1,
		},
		Ownership: OwnershipPolicy{Kind: OwnershipShared},
		Partition: PartitionPolicy{Names: []string{"scada"}},
	}
}

// QoSCommand returns QoS for control commands.
// Reliable delivery, transient-local durability, no deadline.
func QoSCommand() QoSProfile {
	return QoSProfile{
		Reliability: ReliabilityPolicy{
			Kind:            ReliabilityReliable,
			MaxBlockingTime: FromGoDuration(10 * time.Second),
		},
		Durability: DurabilityPolicy{Kind: DurabilityTransientLocal},
		Deadline:   DeadlinePolicy{Period: DurationInfinite},
		Lifespan:   LifespanPolicy{Duration: FromGoDuration(30 * time.Second)},
		History: HistoryPolicy{
			Kind:  HistoryKeepAll,
			Depth: 50,
		},
		Ownership: OwnershipPolicy{Kind: OwnershipShared},
		Partition: PartitionPolicy{Names: []string{"scada"}},
	}
}

// QoSForTopic returns the appropriate QoS profile for a given DDS topic.
func QoSForTopic(topic DDSTopic) QoSProfile {
	switch topic {
	case TopicSolarAlarm, TopicWaterAlarm:
		return QoSAlarm()
	case TopicSolarHeartbeat, TopicWaterHeartbeat:
		return QoSHeartbeat()
	case TopicSolarConfig, TopicWaterConfig:
		return QoSCommand()
	default:
		return QoSSensorData()
	}
}

// ═══════════════════════════════════════════════
// QoS Parameter IDs (RTPS wire format)
// ═══════════════════════════════════════════════

// RTPS Parameter IDs for inline QoS
const (
	PIDReliability    uint16 = 0x001A
	PIDDurability     uint16 = 0x001D
	PIDDeadline       uint16 = 0x0023
	PIDLifespan       uint16 = 0x002B
	PIDHistory        uint16 = 0x0040
	PIDOwnership      uint16 = 0x001F
	PIDPartition      uint16 = 0x0029
	PIDSentinel       uint16 = 0x0001
	PIDTopicName      uint16 = 0x0005
	PIDTypeName       uint16 = 0x0007
	PIDEndpointGUID   uint16 = 0x005A
	PIDKeyHash        uint16 = 0x0070
	PIDStatusInfo     uint16 = 0x0071
)

// EncodeQoSInline serializes QoS policies as RTPS inline QoS parameters
func EncodeQoSInline(qos QoSProfile) []byte {
	buf := make([]byte, 0, 128)
	endian := binary.LittleEndian

	// Reliability
	buf = appendParameter(buf, PIDReliability, 12, endian, func(b []byte) {
		endian.PutUint32(b[0:4], uint32(qos.Reliability.Kind))
		endian.PutUint32(b[4:8], uint32(qos.Reliability.MaxBlockingTime.Seconds))
		endian.PutUint32(b[8:12], qos.Reliability.MaxBlockingTime.Fraction)
	})

	// Durability
	buf = appendParameter(buf, PIDDurability, 4, endian, func(b []byte) {
		endian.PutUint32(b[0:4], uint32(qos.Durability.Kind))
	})

	// Deadline
	if !qos.Deadline.Period.IsInfinite() {
		buf = appendParameter(buf, PIDDeadline, 8, endian, func(b []byte) {
			endian.PutUint32(b[0:4], uint32(qos.Deadline.Period.Seconds))
			endian.PutUint32(b[4:8], qos.Deadline.Period.Fraction)
		})
	}

	// History
	buf = appendParameter(buf, PIDHistory, 8, endian, func(b []byte) {
		endian.PutUint32(b[0:4], uint32(qos.History.Kind))
		endian.PutUint32(b[4:8], uint32(qos.History.Depth))
	})

	// Sentinel (marks end of parameter list)
	sentinel := make([]byte, 4)
	endian.PutUint16(sentinel[0:2], PIDSentinel)
	endian.PutUint16(sentinel[2:4], 0)
	buf = append(buf, sentinel...)

	return buf
}

// appendParameter adds a single RTPS parameter to the buffer
func appendParameter(buf []byte, pid uint16, length uint16, endian binary.ByteOrder, fill func([]byte)) []byte {
	// Pad length to 4-byte boundary
	paddedLen := (length + 3) & ^uint16(3)

	header := make([]byte, 4)
	endian.PutUint16(header[0:2], pid)
	endian.PutUint16(header[2:4], paddedLen)

	data := make([]byte, paddedLen)
	fill(data)

	buf = append(buf, header...)
	buf = append(buf, data...)
	return buf
}

// DecodeQoSParameters parses inline QoS parameters from an RTPS submessage
func DecodeQoSParameters(data []byte, endian binary.ByteOrder) (*QoSProfile, error) {
	qos := &QoSProfile{
		Reliability: ReliabilityPolicy{Kind: ReliabilityBestEffort, MaxBlockingTime: DurationZero},
		Durability:  DurabilityPolicy{Kind: DurabilityVolatile},
		Deadline:    DeadlinePolicy{Period: DurationInfinite},
		Lifespan:    LifespanPolicy{Duration: DurationInfinite},
		History:     HistoryPolicy{Kind: HistoryKeepLast, Depth: 1},
		Ownership:   OwnershipPolicy{Kind: OwnershipShared},
	}

	offset := 0
	for offset+4 <= len(data) {
		pid := endian.Uint16(data[offset:])
		length := endian.Uint16(data[offset+2:])
		offset += 4

		if pid == PIDSentinel {
			break
		}

		if offset+int(length) > len(data) {
			break
		}

		paramData := data[offset : offset+int(length)]

		switch pid {
		case PIDReliability:
			if len(paramData) >= 12 {
				qos.Reliability.Kind = ReliabilityKind(endian.Uint32(paramData[0:4]))
				qos.Reliability.MaxBlockingTime.Seconds = int32(endian.Uint32(paramData[4:8]))
				qos.Reliability.MaxBlockingTime.Fraction = endian.Uint32(paramData[8:12])
			}
		case PIDDurability:
			if len(paramData) >= 4 {
				qos.Durability.Kind = DurabilityKind(endian.Uint32(paramData[0:4]))
			}
		case PIDDeadline:
			if len(paramData) >= 8 {
				qos.Deadline.Period.Seconds = int32(endian.Uint32(paramData[0:4]))
				qos.Deadline.Period.Fraction = endian.Uint32(paramData[4:8])
			}
		case PIDLifespan:
			if len(paramData) >= 8 {
				qos.Lifespan.Duration.Seconds = int32(endian.Uint32(paramData[0:4]))
				qos.Lifespan.Duration.Fraction = endian.Uint32(paramData[4:8])
			}
		case PIDHistory:
			if len(paramData) >= 8 {
				qos.History.Kind = HistoryKind(endian.Uint32(paramData[0:4]))
				qos.History.Depth = int32(endian.Uint32(paramData[4:8]))
			}
		case PIDOwnership:
			if len(paramData) >= 4 {
				qos.Ownership.Kind = OwnershipKind(endian.Uint32(paramData[0:4]))
			}
		}

		offset += int(length)
	}

	return qos, nil
}

// IsQoSCompatible checks if a reader QoS is compatible with a writer QoS.
// This implements a subset of the DDS QoS compatibility rules.
func IsQoSCompatible(reader, writer QoSProfile) bool {
	// Reliability: a RELIABLE reader requires a RELIABLE writer
	if reader.Reliability.Kind == ReliabilityReliable &&
		writer.Reliability.Kind == ReliabilityBestEffort {
		return false
	}

	// Durability: reader durability must not exceed writer durability
	if reader.Durability.Kind > writer.Durability.Kind {
		return false
	}

	// Ownership: both must agree
	if reader.Ownership.Kind != writer.Ownership.Kind {
		return false
	}

	return true
}
