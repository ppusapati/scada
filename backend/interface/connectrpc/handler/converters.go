package handler

import (
	"context"

	"connectrpc.com/connect"
	"google.golang.org/protobuf/types/known/timestamppb"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	scadav1 "scada-system/gen/scada/v1"
)

// ── Proto ↔ Entity Converters ──

func entityModeToProto(m entity.OperatingMode) scadav1.OperatingMode {
	switch m {
	case entity.ModeAuto:
		return scadav1.OperatingMode_OPERATING_MODE_AUTO
	case entity.ModeManual:
		return scadav1.OperatingMode_OPERATING_MODE_MANUAL
	case entity.ModeMaintenance:
		return scadav1.OperatingMode_OPERATING_MODE_MAINTENANCE
	default:
		return scadav1.OperatingMode_OPERATING_MODE_UNSPECIFIED
	}
}

func protoModeToEntity(m scadav1.OperatingMode) entity.OperatingMode {
	switch m {
	case scadav1.OperatingMode_OPERATING_MODE_AUTO:
		return entity.ModeAuto
	case scadav1.OperatingMode_OPERATING_MODE_MANUAL:
		return entity.ModeManual
	case scadav1.OperatingMode_OPERATING_MODE_MAINTENANCE:
		return entity.ModeMaintenance
	default:
		return entity.ModeAuto
	}
}

func deviceStatusToProto(s entity.DeviceStatus) scadav1.DeviceStatus {
	switch s {
	case entity.DeviceOnline:
		return scadav1.DeviceStatus_DEVICE_STATUS_ONLINE
	case entity.DeviceOffline:
		return scadav1.DeviceStatus_DEVICE_STATUS_OFFLINE
	case entity.DeviceFault:
		return scadav1.DeviceStatus_DEVICE_STATUS_FAULT
	case entity.DeviceMaintenance:
		return scadav1.DeviceStatus_DEVICE_STATUS_MAINTENANCE
	default:
		return scadav1.DeviceStatus_DEVICE_STATUS_UNSPECIFIED
	}
}

func devicesToProto(devices []entity.Device) []*scadav1.Device {
	result := make([]*scadav1.Device, len(devices))
	for i, d := range devices {
		result[i] = &scadav1.Device{
			Id:         d.ID,
			Name:       d.Name,
			DeviceType: d.DeviceType,
			Location:   d.Location,
			Subsystem:  string(d.Subsystem),
			Status:     deviceStatusToProto(d.Status),
			LastSeen:   timestamppb.New(d.LastSeen),
		}
	}
	return result
}

// ── Error Mapping: Domain errors → ConnectRPC codes ──

func toConnectError(err error) *connect.Error {
	if err == nil {
		return nil
	}

	switch {
	case domainerr.IsNotFound(err):
		return connect.NewError(connect.CodeNotFound, err)
	case domainerr.IsValidation(err):
		return connect.NewError(connect.CodeInvalidArgument, err)
	case domainerr.IsAuthError(err):
		return connect.NewError(connect.CodeUnauthenticated, err)
	case domainerr.IsUnavailable(err):
		return connect.NewError(connect.CodeUnavailable, err)
	default:
		return connect.NewError(connect.CodeInternal, err)
	}
}

// ── Context Helpers ──

type contextKey string

const userIDKey contextKey = "user_id"

func getUserIDFromCtx(ctx context.Context) string {
	if id, ok := ctx.Value(userIDKey).(string); ok {
		return id
	}
	return ""
}
