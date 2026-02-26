package handler

import (
	"context"

	"connectrpc.com/connect"
	"google.golang.org/protobuf/types/known/timestamppb"

	"scada-system/domain/entity"
	domainerr "scada-system/domain/errors"
	scadav1 "scada-system/gen/scada/v1"
	"scada-system/gen/scada/v1/scadav1connect"
	wateruc "scada-system/usecase/water"
)

// WaterServer implements the ConnectRPC WaterService
type WaterServer struct {
	scadav1connect.UnimplementedWaterServiceHandler
	uc *wateruc.UseCase
}

func NewWaterServer(uc *wateruc.UseCase) *WaterServer {
	return &WaterServer{uc: uc}
}

func (s *WaterServer) GetSystems(ctx context.Context, req *connect.Request[scadav1.GetWaterSystemsRequest]) (*connect.Response[scadav1.GetWaterSystemsResponse], error) {
	systems, err := s.uc.GetAllSystems(ctx)
	if err != nil {
		return nil, toConnectError(err)
	}

	pbSystems := make([]*scadav1.WaterSystem, len(systems))
	for i, sys := range systems {
		pbSystems[i] = waterEntityToProto(&sys)
	}

	return connect.NewResponse(&scadav1.GetWaterSystemsResponse{
		Systems: pbSystems,
	}), nil
}

func (s *WaterServer) GetSystem(ctx context.Context, req *connect.Request[scadav1.GetWaterSystemRequest]) (*connect.Response[scadav1.WaterSystemResponse], error) {
	system, err := s.uc.GetSystem(ctx, req.Msg.SystemId)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.WaterSystemResponse{
		System: waterEntityToProto(system),
	}), nil
}

func (s *WaterServer) ControlPump(ctx context.Context, req *connect.Request[scadav1.ControlPumpRequest]) (*connect.Response[scadav1.CommandResponse], error) {
	userID := getUserIDFromCtx(ctx)
	if err := s.uc.ControlPump(ctx, req.Msg.SystemId, req.Msg.Action, userID); err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.CommandResponse{
		Success: true,
		Message: "pump " + req.Msg.Action + " command sent",
	}), nil
}

func (s *WaterServer) ControlValve(ctx context.Context, req *connect.Request[scadav1.ControlValveRequest]) (*connect.Response[scadav1.CommandResponse], error) {
	userID := getUserIDFromCtx(ctx)
	if err := s.uc.ControlValve(ctx, req.Msg.SystemId, req.Msg.ValveId, req.Msg.Position, userID); err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.CommandResponse{
		Success: true,
		Message: "valve command sent",
	}), nil
}

func (s *WaterServer) SetMode(ctx context.Context, req *connect.Request[scadav1.SetWaterModeRequest]) (*connect.Response[scadav1.CommandResponse], error) {
	mode := protoModeToEntity(req.Msg.Mode)
	if err := s.uc.SetOperatingMode(ctx, req.Msg.SystemId, mode); err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.CommandResponse{
		Success: true,
		Message: "mode updated",
	}), nil
}

func (s *WaterServer) GetDevices(ctx context.Context, req *connect.Request[scadav1.GetDevicesRequest]) (*connect.Response[scadav1.GetDevicesResponse], error) {
	devices, err := s.uc.GetDevices(ctx)
	if err != nil {
		return nil, toConnectError(err)
	}

	return connect.NewResponse(&scadav1.GetDevicesResponse{
		Devices: devicesToProto(devices),
	}), nil
}

func waterEntityToProto(w *entity.WaterSystem) *scadav1.WaterSystem {
	return &scadav1.WaterSystem{
		Id:            w.ID,
		Name:          w.Name,
		TankLevel:     w.TankLevel,
		FlowRateIn:    w.FlowRateIn,
		FlowRateOut:   w.FlowRateOut,
		Pressure:      w.Pressure,
		PhLevel:       w.PHLevel,
		Turbidity:     w.Turbidity,
		ChlorineLevel: w.ChlorineLevel,
		PumpStatus:    string(w.PumpStatus),
		OperatingMode: entityModeToProto(w.OperatingMode),
		UpdatedAt:     timestamppb.New(w.UpdatedAt),
	}
}
