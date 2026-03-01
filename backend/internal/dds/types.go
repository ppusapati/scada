package dds

import (
	"encoding/binary"
	"fmt"
	"math"
	"time"
)

// ═══════════════════════════════════════════════
// CDR (Common Data Representation) Serialization
// ═══════════════════════════════════════════════

// CDREndianness represents byte ordering for CDR encoding
type CDREndianness byte

const (
	CDRBigEndian    CDREndianness = 0x00
	CDRLittleEndian CDREndianness = 0x01
)

// CDREncoder serializes Go structs into CDR wire format
type CDREncoder struct {
	buf    []byte
	endian binary.ByteOrder
	offset int
}

// NewCDREncoder creates a new CDR encoder with a 4-byte encapsulation header
func NewCDREncoder() *CDREncoder {
	enc := &CDREncoder{
		buf:    make([]byte, 0, 256),
		endian: binary.LittleEndian,
		offset: 0,
	}
	// CDR encapsulation header: {0x00, endianness, 0x00, 0x00}
	enc.buf = append(enc.buf, 0x00, byte(CDRLittleEndian), 0x00, 0x00)
	enc.offset = 4
	return enc
}

func (e *CDREncoder) align(n int) {
	padding := (n - (e.offset % n)) % n
	for i := 0; i < padding; i++ {
		e.buf = append(e.buf, 0x00)
		e.offset++
	}
}

// WriteUint8 writes a single byte
func (e *CDREncoder) WriteUint8(v uint8) {
	e.buf = append(e.buf, v)
	e.offset++
}

// WriteUint16 writes a 16-bit unsigned integer with alignment
func (e *CDREncoder) WriteUint16(v uint16) {
	e.align(2)
	b := make([]byte, 2)
	e.endian.PutUint16(b, v)
	e.buf = append(e.buf, b...)
	e.offset += 2
}

// WriteInt32 writes a 32-bit signed integer with alignment
func (e *CDREncoder) WriteInt32(v int32) {
	e.align(4)
	b := make([]byte, 4)
	e.endian.PutUint32(b, uint32(v))
	e.buf = append(e.buf, b...)
	e.offset += 4
}

// WriteUint32 writes a 32-bit unsigned integer with alignment
func (e *CDREncoder) WriteUint32(v uint32) {
	e.align(4)
	b := make([]byte, 4)
	e.endian.PutUint32(b, v)
	e.buf = append(e.buf, b...)
	e.offset += 4
}

// WriteInt64 writes a 64-bit signed integer with alignment
func (e *CDREncoder) WriteInt64(v int64) {
	e.align(8)
	b := make([]byte, 8)
	e.endian.PutUint64(b, uint64(v))
	e.buf = append(e.buf, b...)
	e.offset += 8
}

// WriteUint64 writes a 64-bit unsigned integer with alignment
func (e *CDREncoder) WriteUint64(v uint64) {
	e.align(8)
	b := make([]byte, 8)
	e.endian.PutUint64(b, v)
	e.buf = append(e.buf, b...)
	e.offset += 8
}

// WriteFloat32 writes a 32-bit float with alignment
func (e *CDREncoder) WriteFloat32(v float32) {
	e.WriteUint32(math.Float32bits(v))
}

// WriteFloat64 writes a 64-bit float with alignment
func (e *CDREncoder) WriteFloat64(v float64) {
	e.WriteUint64(math.Float64bits(v))
}

// WriteBool writes a boolean as a single byte
func (e *CDREncoder) WriteBool(v bool) {
	if v {
		e.WriteUint8(1)
	} else {
		e.WriteUint8(0)
	}
}

// WriteString writes a CDR string (4-byte length prefix including null terminator, then data + null)
func (e *CDREncoder) WriteString(s string) {
	length := uint32(len(s) + 1) // include null terminator
	e.WriteUint32(length)
	e.buf = append(e.buf, []byte(s)...)
	e.buf = append(e.buf, 0x00) // null terminator
	e.offset += len(s) + 1
}

// WriteTimestamp writes a DDS Time_t (seconds + fraction as two int32s)
func (e *CDREncoder) WriteTimestamp(t time.Time) {
	e.WriteInt32(int32(t.Unix()))
	e.WriteUint32(uint32(t.Nanosecond()))
}

// Bytes returns the encoded CDR data
func (e *CDREncoder) Bytes() []byte {
	return e.buf
}

// CDRDecoder deserializes CDR-encoded bytes into Go values
type CDRDecoder struct {
	data   []byte
	endian binary.ByteOrder
	offset int
}

// NewCDRDecoder creates a decoder, reading the 4-byte encapsulation header
func NewCDRDecoder(data []byte) (*CDRDecoder, error) {
	if len(data) < 4 {
		return nil, fmt.Errorf("CDR data too short: %d bytes", len(data))
	}
	d := &CDRDecoder{
		data:   data,
		offset: 4, // skip encapsulation header
	}
	// data[1] encodes endianness
	if data[1] == byte(CDRLittleEndian) {
		d.endian = binary.LittleEndian
	} else {
		d.endian = binary.BigEndian
	}
	return d, nil
}

func (d *CDRDecoder) remaining() int {
	return len(d.data) - d.offset
}

func (d *CDRDecoder) align(n int) error {
	padding := (n - (d.offset % n)) % n
	if d.remaining() < padding {
		return fmt.Errorf("CDR align: not enough data for %d-byte alignment at offset %d", n, d.offset)
	}
	d.offset += padding
	return nil
}

// ReadUint8 reads a single byte
func (d *CDRDecoder) ReadUint8() (uint8, error) {
	if d.remaining() < 1 {
		return 0, fmt.Errorf("CDR ReadUint8: need 1 byte, have %d", d.remaining())
	}
	v := d.data[d.offset]
	d.offset++
	return v, nil
}

// ReadUint16 reads a 16-bit unsigned integer
func (d *CDRDecoder) ReadUint16() (uint16, error) {
	if err := d.align(2); err != nil {
		return 0, err
	}
	if d.remaining() < 2 {
		return 0, fmt.Errorf("CDR ReadUint16: need 2 bytes, have %d", d.remaining())
	}
	v := d.endian.Uint16(d.data[d.offset:])
	d.offset += 2
	return v, nil
}

// ReadInt32 reads a 32-bit signed integer
func (d *CDRDecoder) ReadInt32() (int32, error) {
	if err := d.align(4); err != nil {
		return 0, err
	}
	if d.remaining() < 4 {
		return 0, fmt.Errorf("CDR ReadInt32: need 4 bytes, have %d", d.remaining())
	}
	v := d.endian.Uint32(d.data[d.offset:])
	d.offset += 4
	return int32(v), nil
}

// ReadUint32 reads a 32-bit unsigned integer
func (d *CDRDecoder) ReadUint32() (uint32, error) {
	if err := d.align(4); err != nil {
		return 0, err
	}
	if d.remaining() < 4 {
		return 0, fmt.Errorf("CDR ReadUint32: need 4 bytes, have %d", d.remaining())
	}
	v := d.endian.Uint32(d.data[d.offset:])
	d.offset += 4
	return v, nil
}

// ReadInt64 reads a 64-bit signed integer
func (d *CDRDecoder) ReadInt64() (int64, error) {
	if err := d.align(8); err != nil {
		return 0, err
	}
	if d.remaining() < 8 {
		return 0, fmt.Errorf("CDR ReadInt64: need 8 bytes, have %d", d.remaining())
	}
	v := d.endian.Uint64(d.data[d.offset:])
	d.offset += 8
	return int64(v), nil
}

// ReadUint64 reads a 64-bit unsigned integer
func (d *CDRDecoder) ReadUint64() (uint64, error) {
	if err := d.align(8); err != nil {
		return 0, err
	}
	if d.remaining() < 8 {
		return 0, fmt.Errorf("CDR ReadUint64: need 8 bytes, have %d", d.remaining())
	}
	v := d.endian.Uint64(d.data[d.offset:])
	d.offset += 8
	return v, nil
}

// ReadFloat32 reads a 32-bit float
func (d *CDRDecoder) ReadFloat32() (float32, error) {
	bits, err := d.ReadUint32()
	if err != nil {
		return 0, err
	}
	return math.Float32frombits(bits), nil
}

// ReadFloat64 reads a 64-bit float
func (d *CDRDecoder) ReadFloat64() (float64, error) {
	bits, err := d.ReadUint64()
	if err != nil {
		return 0, err
	}
	return math.Float64frombits(bits), nil
}

// ReadBool reads a boolean
func (d *CDRDecoder) ReadBool() (bool, error) {
	v, err := d.ReadUint8()
	if err != nil {
		return false, err
	}
	return v != 0, nil
}

// ReadString reads a CDR string (length-prefixed with null terminator)
func (d *CDRDecoder) ReadString() (string, error) {
	length, err := d.ReadUint32()
	if err != nil {
		return "", fmt.Errorf("CDR ReadString length: %w", err)
	}
	if length == 0 {
		return "", nil
	}
	if d.remaining() < int(length) {
		return "", fmt.Errorf("CDR ReadString: need %d bytes, have %d", length, d.remaining())
	}
	// length includes null terminator
	s := string(d.data[d.offset : d.offset+int(length)-1])
	d.offset += int(length)
	return s, nil
}

// ReadTimestamp reads a DDS Time_t (seconds + nanoseconds)
func (d *CDRDecoder) ReadTimestamp() (time.Time, error) {
	sec, err := d.ReadInt32()
	if err != nil {
		return time.Time{}, fmt.Errorf("CDR ReadTimestamp sec: %w", err)
	}
	nsec, err := d.ReadUint32()
	if err != nil {
		return time.Time{}, fmt.Errorf("CDR ReadTimestamp nsec: %w", err)
	}
	return time.Unix(int64(sec), int64(nsec)), nil
}

// ═══════════════════════════════════════════════
// DDS Data Type Definitions (matching IDL)
// ═══════════════════════════════════════════════

// Timestamp represents a DDS timestamp (seconds + nanoseconds)
type Timestamp struct {
	Seconds     int32  `json:"seconds"`
	Nanoseconds uint32 `json:"nanoseconds"`
}

// ToTime converts a DDS Timestamp to Go time.Time
func (t Timestamp) ToTime() time.Time {
	return time.Unix(int64(t.Seconds), int64(t.Nanoseconds))
}

// TimestampFromTime creates a DDS Timestamp from Go time.Time
func TimestampFromTime(t time.Time) Timestamp {
	return Timestamp{
		Seconds:     int32(t.Unix()),
		Nanoseconds: uint32(t.Nanosecond()),
	}
}

// DeviceIdentity identifies a device in the DDS domain
type DeviceIdentity struct {
	DeviceID  string `json:"device_id"`
	Subsystem string `json:"subsystem"` // "solar" or "water"
	Location  string `json:"location"`
}

func (di *DeviceIdentity) Encode(enc *CDREncoder) {
	enc.WriteString(di.DeviceID)
	enc.WriteString(di.Subsystem)
	enc.WriteString(di.Location)
}

func (di *DeviceIdentity) Decode(dec *CDRDecoder) error {
	var err error
	if di.DeviceID, err = dec.ReadString(); err != nil {
		return fmt.Errorf("DeviceIdentity.DeviceID: %w", err)
	}
	if di.Subsystem, err = dec.ReadString(); err != nil {
		return fmt.Errorf("DeviceIdentity.Subsystem: %w", err)
	}
	if di.Location, err = dec.ReadString(); err != nil {
		return fmt.Errorf("DeviceIdentity.Location: %w", err)
	}
	return nil
}

// ═══════════════════════════════════════════════
// Solar Subsystem Data Types
// ═══════════════════════════════════════════════

// StringVoltageData represents voltage measurements from a solar string
type StringVoltageData struct {
	Device    DeviceIdentity `json:"device"`
	Timestamp Timestamp      `json:"timestamp"`
	StringID  uint16         `json:"string_id"`
	Voltage   float32        `json:"voltage"`    // Volts
	Quality   uint8          `json:"quality"`     // 0=good, 1=uncertain, 2=bad
}

func (d *StringVoltageData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteUint16(d.StringID)
	enc.WriteFloat32(d.Voltage)
	enc.WriteUint8(d.Quality)
}

func (d *StringVoltageData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return fmt.Errorf("StringVoltageData.Timestamp.Seconds: %w", err)
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return fmt.Errorf("StringVoltageData.Timestamp.Nanoseconds: %w", err)
	}
	if d.StringID, err = dec.ReadUint16(); err != nil {
		return fmt.Errorf("StringVoltageData.StringID: %w", err)
	}
	if d.Voltage, err = dec.ReadFloat32(); err != nil {
		return fmt.Errorf("StringVoltageData.Voltage: %w", err)
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return fmt.Errorf("StringVoltageData.Quality: %w", err)
	}
	return nil
}

// StringCurrentData represents current measurements from a solar string
type StringCurrentData struct {
	Device    DeviceIdentity `json:"device"`
	Timestamp Timestamp      `json:"timestamp"`
	StringID  uint16         `json:"string_id"`
	Current   float32        `json:"current"` // Amps
	Quality   uint8          `json:"quality"`
}

func (d *StringCurrentData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteUint16(d.StringID)
	enc.WriteFloat32(d.Current)
	enc.WriteUint8(d.Quality)
}

func (d *StringCurrentData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.StringID, err = dec.ReadUint16(); err != nil {
		return err
	}
	if d.Current, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// BusPowerData represents DC bus power measurements
type BusPowerData struct {
	Device       DeviceIdentity `json:"device"`
	Timestamp    Timestamp      `json:"timestamp"`
	BusVoltage   float32        `json:"bus_voltage"`   // Volts
	BusCurrent   float32        `json:"bus_current"`   // Amps
	Power        float32        `json:"power"`         // kW
	DailyEnergy  float64        `json:"daily_energy"`  // kWh
	TotalEnergy  float64        `json:"total_energy"`  // MWh
	Efficiency   float32        `json:"efficiency"`    // percentage
	Quality      uint8          `json:"quality"`
}

func (d *BusPowerData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteFloat32(d.BusVoltage)
	enc.WriteFloat32(d.BusCurrent)
	enc.WriteFloat32(d.Power)
	enc.WriteFloat64(d.DailyEnergy)
	enc.WriteFloat64(d.TotalEnergy)
	enc.WriteFloat32(d.Efficiency)
	enc.WriteUint8(d.Quality)
}

func (d *BusPowerData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.BusVoltage, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.BusCurrent, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Power, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.DailyEnergy, err = dec.ReadFloat64(); err != nil {
		return err
	}
	if d.TotalEnergy, err = dec.ReadFloat64(); err != nil {
		return err
	}
	if d.Efficiency, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// IrradianceData represents solar irradiance sensor data
type IrradianceData struct {
	Device     DeviceIdentity `json:"device"`
	Timestamp  Timestamp      `json:"timestamp"`
	Irradiance float32        `json:"irradiance"` // W/m^2
	Quality    uint8          `json:"quality"`
}

func (d *IrradianceData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteFloat32(d.Irradiance)
	enc.WriteUint8(d.Quality)
}

func (d *IrradianceData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.Irradiance, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// TemperatureData represents module temperature measurements
type TemperatureData struct {
	Device      DeviceIdentity `json:"device"`
	Timestamp   Timestamp      `json:"timestamp"`
	Temperature float32        `json:"temperature"` // degrees Celsius
	SensorID    uint16         `json:"sensor_id"`
	Quality     uint8          `json:"quality"`
}

func (d *TemperatureData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteFloat32(d.Temperature)
	enc.WriteUint16(d.SensorID)
	enc.WriteUint8(d.Quality)
}

func (d *TemperatureData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.Temperature, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.SensorID, err = dec.ReadUint16(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// ═══════════════════════════════════════════════
// Water Subsystem Data Types
// ═══════════════════════════════════════════════

// WaterLevelData represents water tank level measurement
type WaterLevelData struct {
	Device    DeviceIdentity `json:"device"`
	Timestamp Timestamp      `json:"timestamp"`
	Level     float32        `json:"level"` // percentage 0-100
	Volume    float32        `json:"volume"` // liters
	Quality   uint8          `json:"quality"`
}

func (d *WaterLevelData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteFloat32(d.Level)
	enc.WriteFloat32(d.Volume)
	enc.WriteUint8(d.Quality)
}

func (d *WaterLevelData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.Level, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Volume, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// FlowRateData represents flow measurement from a flow meter
type FlowRateData struct {
	Device     DeviceIdentity `json:"device"`
	Timestamp  Timestamp      `json:"timestamp"`
	FlowRate   float32        `json:"flow_rate"`   // L/min
	TotalFlow  float64        `json:"total_flow"`  // total liters
	Direction  uint8          `json:"direction"`   // 0=in, 1=out
	Quality    uint8          `json:"quality"`
}

func (d *FlowRateData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteFloat32(d.FlowRate)
	enc.WriteFloat64(d.TotalFlow)
	enc.WriteUint8(d.Direction)
	enc.WriteUint8(d.Quality)
}

func (d *FlowRateData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.FlowRate, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.TotalFlow, err = dec.ReadFloat64(); err != nil {
		return err
	}
	if d.Direction, err = dec.ReadUint8(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// PressureData represents pressure measurement
type PressureData struct {
	Device    DeviceIdentity `json:"device"`
	Timestamp Timestamp      `json:"timestamp"`
	Pressure  float32        `json:"pressure"` // bar
	SensorID  uint16         `json:"sensor_id"`
	Quality   uint8          `json:"quality"`
}

func (d *PressureData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteFloat32(d.Pressure)
	enc.WriteUint16(d.SensorID)
	enc.WriteUint8(d.Quality)
}

func (d *PressureData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.Pressure, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.SensorID, err = dec.ReadUint16(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// WaterQualityData represents water quality measurements (pH, turbidity, chlorine)
type WaterQualityData struct {
	Device        DeviceIdentity `json:"device"`
	Timestamp     Timestamp      `json:"timestamp"`
	PHLevel       float32        `json:"ph_level"`
	Turbidity     float32        `json:"turbidity"`      // NTU
	ChlorineLevel float32        `json:"chlorine_level"`  // mg/L
	Conductivity  float32        `json:"conductivity"`    // uS/cm
	Quality       uint8          `json:"quality"`
}

func (d *WaterQualityData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteFloat32(d.PHLevel)
	enc.WriteFloat32(d.Turbidity)
	enc.WriteFloat32(d.ChlorineLevel)
	enc.WriteFloat32(d.Conductivity)
	enc.WriteUint8(d.Quality)
}

func (d *WaterQualityData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.PHLevel, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Turbidity, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.ChlorineLevel, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Conductivity, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// ═══════════════════════════════════════════════
// Shared Data Types (Alarm, Heartbeat, Config)
// ═══════════════════════════════════════════════

// AlarmSeverityDDS mirrors the DDS IDL alarm severity enum
type AlarmSeverityDDS uint32

const (
	AlarmSeverityInfo     AlarmSeverityDDS = 0
	AlarmSeverityWarning  AlarmSeverityDDS = 1
	AlarmSeverityCritical AlarmSeverityDDS = 2
)

func (s AlarmSeverityDDS) String() string {
	switch s {
	case AlarmSeverityInfo:
		return "info"
	case AlarmSeverityWarning:
		return "warning"
	case AlarmSeverityCritical:
		return "critical"
	default:
		return "unknown"
	}
}

// AlarmEvent represents a DDS alarm notification
type AlarmEvent struct {
	Device    DeviceIdentity   `json:"device"`
	Timestamp Timestamp        `json:"timestamp"`
	AlarmID   uint32           `json:"alarm_id"`
	Severity  AlarmSeverityDDS `json:"severity"`
	Category  string           `json:"category"`
	Message   string           `json:"message"`
	Value     float64          `json:"value"`
	Threshold float64          `json:"threshold"`
	Active    bool             `json:"active"`
}

func (d *AlarmEvent) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteUint32(d.AlarmID)
	enc.WriteUint32(uint32(d.Severity))
	enc.WriteString(d.Category)
	enc.WriteString(d.Message)
	enc.WriteFloat64(d.Value)
	enc.WriteFloat64(d.Threshold)
	enc.WriteBool(d.Active)
}

func (d *AlarmEvent) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.AlarmID, err = dec.ReadUint32(); err != nil {
		return err
	}
	sev, err := dec.ReadUint32()
	if err != nil {
		return err
	}
	d.Severity = AlarmSeverityDDS(sev)
	if d.Category, err = dec.ReadString(); err != nil {
		return err
	}
	if d.Message, err = dec.ReadString(); err != nil {
		return err
	}
	if d.Value, err = dec.ReadFloat64(); err != nil {
		return err
	}
	if d.Threshold, err = dec.ReadFloat64(); err != nil {
		return err
	}
	if d.Active, err = dec.ReadBool(); err != nil {
		return err
	}
	return nil
}

// PumpStatusDDS represents pump operational state in DDS
type PumpStatusDDS uint32

const (
	PumpStatusStopped PumpStatusDDS = 0
	PumpStatusRunning PumpStatusDDS = 1
	PumpStatusFault   PumpStatusDDS = 2
)

func (p PumpStatusDDS) String() string {
	switch p {
	case PumpStatusStopped:
		return "stopped"
	case PumpStatusRunning:
		return "running"
	case PumpStatusFault:
		return "fault"
	default:
		return "unknown"
	}
}

// PumpStatusData represents pump status from the water subsystem
type PumpStatusData struct {
	Device    DeviceIdentity `json:"device"`
	Timestamp Timestamp      `json:"timestamp"`
	PumpID    uint16         `json:"pump_id"`
	Status    PumpStatusDDS  `json:"status"`
	RPM       float32        `json:"rpm"`
	Current   float32        `json:"current"` // Amps
	Quality   uint8          `json:"quality"`
}

func (d *PumpStatusData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteUint16(d.PumpID)
	enc.WriteUint32(uint32(d.Status))
	enc.WriteFloat32(d.RPM)
	enc.WriteFloat32(d.Current)
	enc.WriteUint8(d.Quality)
}

func (d *PumpStatusData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.PumpID, err = dec.ReadUint16(); err != nil {
		return err
	}
	st, err := dec.ReadUint32()
	if err != nil {
		return err
	}
	d.Status = PumpStatusDDS(st)
	if d.RPM, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Current, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Quality, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// HeartbeatData represents a device heartbeat message
type HeartbeatData struct {
	Device      DeviceIdentity `json:"device"`
	Timestamp   Timestamp      `json:"timestamp"`
	SequenceNum uint32         `json:"sequence_num"`
	UptimeSec   uint32         `json:"uptime_sec"`
	CPUUsage    float32        `json:"cpu_usage"`  // percentage
	MemUsage    float32        `json:"mem_usage"`  // percentage
	Status      uint8          `json:"status"`     // 0=ok, 1=degraded, 2=error
}

func (d *HeartbeatData) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteUint32(d.SequenceNum)
	enc.WriteUint32(d.UptimeSec)
	enc.WriteFloat32(d.CPUUsage)
	enc.WriteFloat32(d.MemUsage)
	enc.WriteUint8(d.Status)
}

func (d *HeartbeatData) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.SequenceNum, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.UptimeSec, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.CPUUsage, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.MemUsage, err = dec.ReadFloat32(); err != nil {
		return err
	}
	if d.Status, err = dec.ReadUint8(); err != nil {
		return err
	}
	return nil
}

// ConfigCommand represents a configuration/control command sent to embedded devices
type ConfigCommand struct {
	Device      DeviceIdentity    `json:"device"`
	Timestamp   Timestamp         `json:"timestamp"`
	CommandID   uint32            `json:"command_id"`
	Action      string            `json:"action"`
	ParamCount  uint32            `json:"param_count"`
	ParamKeys   []string          `json:"param_keys"`
	ParamValues []string          `json:"param_values"`
	IssuedBy    string            `json:"issued_by"`
	Priority    uint8             `json:"priority"` // 0=low, 1=normal, 2=high, 3=critical
}

func (d *ConfigCommand) Encode(enc *CDREncoder) {
	d.Device.Encode(enc)
	enc.WriteInt32(d.Timestamp.Seconds)
	enc.WriteUint32(d.Timestamp.Nanoseconds)
	enc.WriteUint32(d.CommandID)
	enc.WriteString(d.Action)
	count := uint32(len(d.ParamKeys))
	enc.WriteUint32(count)
	for i := uint32(0); i < count; i++ {
		enc.WriteString(d.ParamKeys[i])
	}
	enc.WriteUint32(count) // CDR sequences have length prefix
	for i := uint32(0); i < count; i++ {
		enc.WriteString(d.ParamValues[i])
	}
	enc.WriteString(d.IssuedBy)
	enc.WriteUint8(d.Priority)
}

func (d *ConfigCommand) Decode(dec *CDRDecoder) error {
	if err := d.Device.Decode(dec); err != nil {
		return err
	}
	var err error
	if d.Timestamp.Seconds, err = dec.ReadInt32(); err != nil {
		return err
	}
	if d.Timestamp.Nanoseconds, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.CommandID, err = dec.ReadUint32(); err != nil {
		return err
	}
	if d.Action, err = dec.ReadString(); err != nil {
		return err
	}
	keyCount, err := dec.ReadUint32()
	if err != nil {
		return err
	}
	d.ParamKeys = make([]string, keyCount)
	for i := uint32(0); i < keyCount; i++ {
		if d.ParamKeys[i], err = dec.ReadString(); err != nil {
			return err
		}
	}
	valCount, err := dec.ReadUint32()
	if err != nil {
		return err
	}
	d.ParamValues = make([]string, valCount)
	for i := uint32(0); i < valCount; i++ {
		if d.ParamValues[i], err = dec.ReadString(); err != nil {
			return err
		}
	}
	if d.IssuedBy, err = dec.ReadString(); err != nil {
		return err
	}
	if d.Priority, err = dec.ReadUint8(); err != nil {
		return err
	}
	d.ParamCount = keyCount
	return nil
}

// ═══════════════════════════════════════════════
// DDS Topic Names
// ═══════════════════════════════════════════════

// DDSTopic is a well-known DDS topic name used in this SCADA system
type DDSTopic string

const (
	// Solar topics
	TopicSolarStringVoltage DDSTopic = "solar/string_voltage"
	TopicSolarStringCurrent DDSTopic = "solar/string_current"
	TopicSolarBusPower      DDSTopic = "solar/bus_power"
	TopicSolarIrradiance    DDSTopic = "solar/irradiance"
	TopicSolarModuleTemp    DDSTopic = "solar/module_temp"
	TopicSolarAlarm         DDSTopic = "solar/alarm"
	TopicSolarHeartbeat     DDSTopic = "solar/heartbeat"
	TopicSolarConfig        DDSTopic = "solar/config"

	// Water topics
	TopicWaterLevel      DDSTopic = "water/level"
	TopicWaterFlow       DDSTopic = "water/flow"
	TopicWaterPressure   DDSTopic = "water/pressure"
	TopicWaterQuality    DDSTopic = "water/quality"
	TopicWaterPumpStatus DDSTopic = "water/pump_status"
	TopicWaterAlarm      DDSTopic = "water/alarm"
	TopicWaterHeartbeat  DDSTopic = "water/heartbeat"
	TopicWaterConfig     DDSTopic = "water/config"
)

// AllSubscribeTopics returns all topics the backend subscribes to
func AllSubscribeTopics() []DDSTopic {
	return []DDSTopic{
		TopicSolarStringVoltage,
		TopicSolarStringCurrent,
		TopicSolarBusPower,
		TopicSolarIrradiance,
		TopicSolarModuleTemp,
		TopicSolarAlarm,
		TopicSolarHeartbeat,
		TopicWaterLevel,
		TopicWaterFlow,
		TopicWaterPressure,
		TopicWaterQuality,
		TopicWaterPumpStatus,
		TopicWaterAlarm,
		TopicWaterHeartbeat,
	}
}

// AllPublishTopics returns all topics the backend publishes commands on
func AllPublishTopics() []DDSTopic {
	return []DDSTopic{
		TopicSolarConfig,
		TopicWaterConfig,
	}
}
